use crate::collections::shared_deque::SharedDeque;
use crate::controller::client::ClientCommand;
use crate::net::protocol::{NetMessage, NetMessageContent, Packet};
use crate::net::snapshot::Applier;
use crate::resources::Resources;
#[allow(unused_imports)]
use log::{debug, error, info};
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

const NB_TRY: u32 = 10;

/// The actual game system that will be running in the main loop
pub struct ClientSystem {
    server_addr: SocketAddr,

    /// Messages incoming from the server.
    from_server: SharedDeque<NetMessage>,

    /// Queue to send to server
    to_server: mpsc::Sender<NetMessage>,

    last_sent_seq_number: u32,
    last_rec_seq_number: u32,
    last_known_state: Option<u8>,

    applier: Applier,

    _rt: tokio::runtime::Runtime,
}

impl ClientSystem {
    pub fn new(server_addr: SocketAddr) -> Self {
        let my_adress = "0.0.0.0:0".parse().unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let mut shared_deque = SharedDeque::new(100);
        let copied_deque = shared_deque.clone();
        let (mut tx, rx) = tokio::sync::mpsc::channel::<NetMessage>(100);

        rt.spawn(async move {
            super::start_server(my_adress, copied_deque, rx).await;
        });

        let last_rec_seq_number = 0;
        let last_known_state = None;

        // Now try to connect to application server
        // TODO

        let mut sent_seq_number = 0;
        // Connection to server. Try to send message every seconds until it receives
        // a connection accepted or a connection refused.
        info!("Will connect to the game server");
        let is_connected: bool = {
            let mut try_nb = 0u32;
            let mut res = false;
            'connection: loop {
                if try_nb >= NB_TRY {
                    info!("Timed out during connection to server");
                    break 'connection;
                }

                tx.try_send(NetMessage {
                    target: server_addr.clone(),
                    content: Packet {
                        content: NetMessageContent::ConnectionRequest,
                        seq_number: sent_seq_number,
                        last_known_state: None,
                    },
                })
                .expect("Error when sending via mpsc channel");
                sent_seq_number += 1;

                thread::sleep(Duration::from_secs(1));
                let evs = shared_deque.drain();
                // ok we might lose some events here. It's alright, the server
                // is sending state every loop and if message needs to be reliably sent,
                // the server will resend it.
                for ev in evs {
                    match ev.content.content {
                        NetMessageContent::ConnectionAccepted => {
                            res = true;
                            break 'connection;
                        }
                        NetMessageContent::ConnectionRefused => {
                            info!("Received connection refused");
                            break 'connection;
                        }
                        _ => error!("Received {:?} when connecting. That is strange", ev),
                    }
                }

                try_nb += 1;
            }

            res
        };

        if !is_connected {
            // TODO better than that.
            panic!("Could not connect");
        }

        Self {
            server_addr,
            from_server: shared_deque,
            to_server: tx,
            last_known_state,
            last_rec_seq_number,
            last_sent_seq_number: sent_seq_number,
            _rt: rt,
            applier: Applier::default(),
        }
    }

    pub fn send_commands(&mut self, commands: &Vec<ClientCommand>) {
        for cmd in commands.iter() {
            self.send_to_server(NetMessageContent::Command(*cmd));
        }

        if commands.is_empty() {
            self.send_to_server(NetMessageContent::Ping);
        }
    }

    fn send_to_server(&mut self, content: NetMessageContent) {
        if let Err(e) = self.to_server.try_send(NetMessage {
            target: self.server_addr.clone(),
            content: Packet {
                content,
                seq_number: self.last_sent_seq_number,
                last_known_state: self.last_known_state,
            },
        }) {
            error!("Error when sending to server = {:?}", e);
        }
        self.last_sent_seq_number += 1;
    }

    /// Will get the latest events that were sent from the server
    pub fn poll_events(&mut self, ecs: &mut hecs::World, resources: &mut Resources) {
        let events = self.from_server.drain();

        for ev in events {
            if self.last_rec_seq_number >= ev.content.seq_number {
                error!(
                    "Received packet out of order: last_rec_seq_number {} > packet.seq_number {}",
                    self.last_rec_seq_number, ev.content.seq_number
                );
            } else {
                self.last_rec_seq_number = ev.content.seq_number;

                if let NetMessageContent::Delta(snapshot) = ev.content.content {
                    if self.last_known_state == snapshot.old_state {
                        debug!("Client received delta: {:?}", snapshot);
                        self.last_known_state = Some(snapshot.new_state);
                        self.applier.apply_latest(ecs, snapshot.delta, resources);
                    }
                }
            }
        }
    }
}
