//! the server side of the game. Keep a list of clients that currently plays together,
//! sends latest state to connected clients, process incoming messages... a lot of stuff.

use crate::collections::option_array::OptionArray;
use crate::collections::shared_deque::SharedDeque;
use crate::net::protocol::{DeltaSnapshotInfo, NetMessage, NetMessageContent, Packet};
use hecs::{Entity, World};
use std::net::SocketAddr;
use tokio::sync::mpsc;

use crate::event::{Event, GameEvent};
use crate::gameplay::player;
use crate::net::snapshot::{SnapshotError, Snapshotter};
use crate::physics::PhysicWorld;
use crate::resources::Resources;
#[allow(unused_imports)]
use log::{debug, error, info, trace};
use shrev::EventChannel;

/// A client connected to the server. Keep the IP address, the latest state known to the client,
/// sequence numbers and so on.
struct Client {
    /// IP/Port of the client.
    addr: SocketAddr,

    // Index in the snapshot circular buffer
    // None is hasn't received information yet
    last_state: Option<u8>,

    // Incremented nb that is sent in the packet
    last_rec_seq_number: u32,
    last_sent_seq_number: u32,

    // The entity in the server ECS associated to this client
    entity: Option<Entity>,
}

/// Server that will run in the main game loop.
pub struct NetworkSystem {
    /// All the clients currently in the game
    my_clients: OptionArray<Client>,

    /// messages coming from the clients the clients (from the async network part).
    from_clients: SharedDeque<NetMessage>,

    /// channel to send messages to the clients.
    to_clients: mpsc::Sender<NetMessage>,

    _rt: tokio::runtime::Runtime,

    snapshotter: Snapshotter,
}

impl NetworkSystem {
    /// Create a new network system. This will also open the sockets :)
    pub fn new(addr: SocketAddr) -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();

        let shared_deque = SharedDeque::new(100);
        let copied_deque = shared_deque.clone();
        let (tx, rx) = tokio::sync::mpsc::channel::<NetMessage>(100);

        rt.spawn(async move {
            super::start_server(addr, copied_deque, rx).await;
        });

        Self {
            from_clients: shared_deque,
            to_clients: tx,
            my_clients: OptionArray::new(8),
            _rt: rt,
            snapshotter: Snapshotter::new(100),
        }
    }

    /// Will fetch the latest messages coming from the clients. Return the game events (move, jump,
    /// ...)
    pub fn poll_events(
        &mut self,
        ecs: &mut hecs::World,
        physics: &mut PhysicWorld,
        resources: &Resources,
    ) -> Vec<(Entity, Event)> {
        let events = self.from_clients.drain();

        let mut game_events = vec![];

        for ev in events {
            trace!("Network system received {:?}", ev);
            if let NetMessageContent::ConnectionRequest = ev.content.content {
                self.handle_connection_request(ev.target, ecs, physics, resources);
            } else {
                // if the client is known, send OK, else send connection refused. Update
                // the last known state so that we send the correct thing in snapshots.
                if let Some(index) = self.get_client_id(ev.target) {
                    let client = self.my_clients.get_mut(index).unwrap();

                    // Discard out of order.
                    if client.last_rec_seq_number >= ev.content.seq_number {
                        error!("Receive packet out of order for {}: last_rec_seq_number {} >= packet.seq_number {}", ev.target, client.last_rec_seq_number, ev.content.seq_number);
                    } else {
                        client.last_state = ev.content.last_known_state;
                        client.last_rec_seq_number = ev.content.seq_number;

                        debug!("Received message from client = {:?}", ev);
                        // Now convert the message as an event that will be processed by the
                        // engine (physics,... and so on).
                        if let Some(ev) = NetworkSystem::handle_client_message(&client, ev.content)
                        {
                            trace!("Will add for processing {:?}", ev);
                            // TODO keep only the latest type of event...
                            game_events.push((client.entity.unwrap().clone(), ev));
                        }
                    }
                } else {
                }
            }
        }

        game_events
    }

    fn handle_client_message(_client: &Client, packet: Packet) -> Option<Event> {
        match packet.content {
            NetMessageContent::Command(cmd) => Some(Event::Client(cmd)),
            _ => {
                trace!("do not process {:?}", packet.content);
                None
            }
        }
    }

    /// This is called when a ConnectionRequest message is received
    /// It will reply with either connection accepted or connection refused
    /// and add the client to our map of clients.
    ///
    /// If a client is already in the map, it should reply connection
    /// accepted. The reason is that the connection acception message
    /// might have been lost so the client thinks it is still trying to connect
    fn handle_connection_request(
        &mut self,
        addr: SocketAddr,
        ecs: &mut hecs::World,
        physics: &mut PhysicWorld,
        resources: &Resources,
    ) {
        info!("New client wants to connect: {:?}", addr);
        info!("Handle new connection request from {}", addr);

        let (to_send, client_id) = {
            if let Some(id) = self.get_client_id(addr) {
                info!("Client was already connected, resend ConnectionAccepted");
                (NetMessageContent::ConnectionAccepted, Some(id))
            } else {
                // in that case we need to find an empty slot. If available,
                // return connection accepted.

                match self.my_clients.add(Client {
                    addr,
                    last_rec_seq_number: 0,
                    last_sent_seq_number: 0,
                    last_state: None,
                    entity: None,
                }) {
                    Some(i) => {
                        info!("New player connected: Player {}!", i);

                        // Now we have a new client, let's create a new player entity
                        // from the player template.
                        let entity = player::spawn_player(ecs, physics, resources);
                        debug!("Player {} entity is {:?}", i, entity);

                        self.my_clients.get_mut(i).unwrap().entity = Some(entity);
                        (NetMessageContent::ConnectionAccepted, Some(i))
                    }

                    None => {
                        info!("Too many clients connected, send ConnectionRefused");
                        (NetMessageContent::ConnectionRefused, None)
                    }
                }
            }
        };

        if let Some(id) = client_id {
            debug!("Send connection accepted");
            self.send_to_client(id, to_send);
        } else {
            // ConnectionRefused is sent to parties that are not client yet.
            if let Err(e) = self.to_clients.try_send(NetMessage {
                target: addr,
                content: Packet {
                    content: to_send,
                    seq_number: 0,
                    last_known_state: None,
                },
            }) {
                error!("Error while sending to client = {:?}", e);
            }
        }
    }

    /// This will send the current state to all clients.
    pub fn send_state(&mut self, ecs: &mut World, resources: &Resources) {
        // First take a snapshot.
        self.snapshotter.set_current(ecs);

        let mut to_disconnect = Vec::new();
        for i in 0..self.my_clients.len() {
            if let Some(client) = self.my_clients.get_mut(i) {
                let client_entity = client.entity.unwrap();
                let delta_res = if let Some(idx) = client.last_state {
                    self.snapshotter.get_delta(idx as usize, ecs, client_entity)
                } else {
                    self.snapshotter.get_full_snapshot(ecs, client_entity)
                };

                match delta_res {
                    Ok(delta) => {
                        debug!("STATE: to player {:?} = {:?}", i, delta);
                        let msg = NetMessageContent::Delta(DeltaSnapshotInfo {
                            delta,
                            old_state: client.last_state,
                            // Don't worry it is ok for now :D
                            new_state: self.snapshotter.get_current_index() as u8,
                        });
                        self.send_to_client(i, msg);
                    }
                    Err(SnapshotError::ClientCaughtUp) => {
                        info!("To disconnect!");
                        to_disconnect.push(i);
                    }
                    Err(e) => error!("{}", e),
                }
            }
        }

        for i in to_disconnect {
            info!("Will disconnect player {}", i);
            if let Some(c) = self.my_clients.remove(i) {
                debug!("Will remove player {}", i);
                if let Some(entity) = c.entity {
                    let mut chan = resources.fetch_mut::<EventChannel<GameEvent>>().unwrap();
                    chan.single_write(GameEvent::Delete(entity));
                    //ecs.despawn(entity);
                }
            } else {
                error!("Could not remove player {}", i);
            }
        }
    }

    /// Should be used to send a message to a client. Will increase a sequence number.
    fn send_to_client(&mut self, client_id: usize, msg: NetMessageContent) {
        let client = self
            .my_clients
            .get_mut(client_id)
            .expect("Something wrong happend here");
        let to_send = NetMessage {
            target: client.addr,
            content: Packet {
                content: msg,
                seq_number: client.last_sent_seq_number,
                last_known_state: None, // doesn't matter on server->client
            },
        };

        if let Err(e) = self.to_clients.try_send(to_send) {
            error!("Error while sending to client = {:?}", e);
        }
        client.last_sent_seq_number += 1;
    }

    fn get_client_id(&self, addr: SocketAddr) -> Option<usize> {
        self.my_clients
            .iter()
            .enumerate()
            .find(|(_, client)| client.is_some() && client.as_ref().unwrap().addr == addr)
            .map(|t| t.0)
    }
}
