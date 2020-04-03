#![allow(warnings)]
use bytes::{Bytes, BytesMut};
use futures::{FutureExt, SinkExt};
use futures_util::stream::StreamExt;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::{io, time};
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::channel;

use r3dtest::collections::shared_deque::SharedDeque;
use r3dtest::net::{
    protocol::{NetMessage, NetMessageContent, Packet},
    start_server,
};

fn main() {
    dotenv::dotenv().ok().unwrap();
    pretty_env_logger::init();
    let mut rt = tokio::runtime::Runtime::new().unwrap();

    let mut shared_deque = SharedDeque::new(100);
    let copied_deque = shared_deque.clone();
    let (mut tx, rx) = tokio::sync::mpsc::channel::<NetMessage>(100);

    let server_addr: SocketAddr = "127.0.0.1:13466".parse().unwrap();

    rt.spawn(async move {
        start_server(server_addr.clone(), copied_deque, rx).await;
    });

    for _ in 0..15 {
        std::thread::sleep(Duration::from_secs(2));

        for msg in shared_deque.drain() {
            println!("{:?}", msg);
        }
    }
}

async fn spawn_server(
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    rx: tokio::sync::mpsc::Receiver<NetMessage>,
) -> Result<(), Box<dyn Error>> {
    // start the client.
    tokio::spawn(async move {
        run_client(client_addr).await;
    });

    let a = UdpSocket::bind(&server_addr).await?;
    let mut socket = UdpFramed::new(a, BytesCodec::default());
    println!("server connected");

    forward_messages(&mut socket, rx).await?;

    Ok(())
}

async fn forward_messages(
    socket: &mut UdpFramed<BytesCodec>,
    mut rx: tokio::sync::mpsc::Receiver<NetMessage>,
) -> Result<(), io::Error> {
    while let Some(i) = rx.recv().await {
        println!("Received message from main loop");
        socket.send(i.pack().unwrap()).await?;
    }
    Ok(())
}

async fn run_client(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let b = UdpSocket::bind(&addr).await?;
    let mut socket = UdpFramed::new(b, BytesCodec::new());

    let timeout = Duration::from_millis(5000);

    println!("Client connected");
    while let Ok(Some(Ok((bytes, addr)))) = time::timeout(timeout, socket.next()).await {
        let recv = NetMessage::unpack(bytes.freeze(), addr).unwrap();
        println!("[b] recv: {:?}", recv);
    }

    Ok(())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // port 0. Will bind to different ports.
    let a = UdpSocket::bind(&addr).await?;
    let b = UdpSocket::bind(&addr).await?;

    let b_addr = b.local_addr()?;

    let mut a = UdpFramed::new(a, BytesCodec::new());
    let mut b = UdpFramed::new(b, BytesCodec::new());
    println!("{:?}", b_addr);

    let ping = ping(&mut a, b_addr);
    let pong = pong(&mut b);
    match futures::future::try_join(ping, pong).await {
        Err(e) => println!("{:?}", e),
        _ => (),
    }
    Ok(())
}

async fn ping(
    socket: &mut UdpFramed<BytesCodec>,
    b_addr: SocketAddr,
) -> Result<(), std::io::Error> {
    let ping_message = NetMessage {
        target: b_addr,
        content: Packet {
            content: NetMessageContent::Ping,
            seq_number: 0,
            last_known_state: None,
        },
    };
    let to_send = ping_message.pack().unwrap();
    socket.send(to_send.clone()).await?;

    for _ in 0..4usize {
        let (bytes, addr) = socket.next().map(|e| e.unwrap()).await?;
        let recv = NetMessage::unpack(bytes.freeze(), addr).unwrap();
        println!("[a] recv: {:?}", recv);
        socket.send(to_send.clone()).await?;
    }

    Ok(())
}

//             `(futures_util::stream::stream::split::SplitSink<&mut tokio_util::udp::frame::UdpFramed<tokio_util::codec::bytes_codec::BytesCodec>, (bytes::bytes::Bytes, std::net::SocketAddr)>, futures_util::stream::stream::split::SplitStream<&mut tokio_util::udp::frame::UdpFramed<tokio_util::codec::bytes_codec::BytesCodec>>)`

type Sink<'a> = futures_util::stream::SplitSink<&'a mut UdpFramed<BytesCodec>, (Bytes, SocketAddr)>;
type Stream<'a> = futures_util::stream::SplitStream<&'a mut UdpFramed<BytesCodec>>;

async fn pong(socket: &mut UdpFramed<BytesCodec>) -> Result<(), io::Error> {
    let timeout = Duration::from_millis(200);

    let (mut sink, mut stream): (Sink, Stream) = socket.split();

    while let Ok(Some(Ok((bytes, addr)))) = time::timeout(timeout, stream.next()).await {
        let recv = NetMessage::unpack(bytes.freeze(), addr).unwrap();
        println!("[b] recv: {:?}", recv);
        let pong_message = NetMessage {
            target: recv.target,
            content: Packet {
                content: NetMessageContent::Ping,
                seq_number: 0,
                last_known_state: None,
            },
        };
        sink.send(pong_message.pack().unwrap()).await?;
    }

    Ok(())
}
