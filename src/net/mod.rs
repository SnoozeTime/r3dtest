//! Contains all the network code (Client, Server, protocol...)
use crate::collections::shared_deque::SharedDeque;
use crate::net::protocol::NetMessage;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use std::net::SocketAddr;
use tokio::sync::mpsc;

use bytes::Bytes;
#[allow(unused_imports)]
use log::{debug, error, info};
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

type Sink = futures_util::stream::SplitSink<UdpFramed<BytesCodec>, (Bytes, SocketAddr)>;
type Stream = futures_util::stream::SplitStream<UdpFramed<BytesCodec>>;

pub mod client;
pub mod protocol;
pub mod server;
pub mod snapshot;

pub async fn start_server(
    addr: SocketAddr,
    from_clients: SharedDeque<NetMessage>,
    to_clients: mpsc::Receiver<NetMessage>,
) {
    info!("Will start UDP on {:?}", addr);
    // Two tasks. One will be listening from messages from the queue. The other will be listening
    // from the socket.
    let socket = UdpSocket::bind(&addr).await.unwrap();
    info!("UDP connected");

    let socket = UdpFramed::new(socket, BytesCodec::new());

    let (sink, stream) = socket.split();

    let listen_incoming_task = listen_incoming(stream, from_clients);
    let forward_messages_task = forward_messages(sink, to_clients);

    match futures::future::try_join(listen_incoming_task, forward_messages_task).await {
        Ok(_) => info!("No problem"),
        Err(e) => error!("Error in server = {:?}", e),
    }
}

async fn listen_incoming(
    mut socket: Stream,
    mut from_clients: SharedDeque<NetMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    while let Some(Ok((bytes, addr))) = socket.next().await {
        debug!("Received message from {:?}", addr);

        match NetMessage::unpack(bytes.freeze(), addr) {
            Ok(msg) => from_clients.push(msg),
            Err(e) => error!("Error while unpacking message = {:?}", e),
        }
    }

    Ok(())
}

async fn forward_messages(
    mut socket: Sink,
    mut to_clients: mpsc::Receiver<NetMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    while let Some(i) = to_clients.recv().await {
        debug!("Received message from main loop = {:?}", i);
        socket.send(i.pack().unwrap()).await?;
    }
    Ok(())
}
