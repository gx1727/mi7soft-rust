use tracing::{debug, error, info};

use crate::common::Command;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc::UnboundedSender;
use std::sync::atomic::{AtomicU64, Ordering};


static TCP_ID: AtomicU64 = AtomicU64::new(1);
static UDP_ID: AtomicU64 = AtomicU64::new(1);


pub async fn run_udp(addr: SocketAddr, tx: UnboundedSender<Command>) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(addr).await?;
    info!("UDP listening on {}", addr);
    let mut buf = vec![0u8; 2048];
    loop {
        let (n, peer) = socket.recv_from(&mut buf).await?;
        let id = UDP_ID.fetch_add(1, Ordering::Relaxed);
        let payload = buf[..n].to_vec();
        let _ = tx.send(Command::UdpPacket { id, peer, payload: payload.clone() });
        // echo
        let _ = socket.send_to(&payload, peer).await;
    }
}