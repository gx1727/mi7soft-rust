use tracing::{debug, error, info};
use crate::common::Command;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::mpsc::UnboundedSender;
use std::sync::atomic::{AtomicU64, Ordering};


static TCP_ID: AtomicU64 = AtomicU64::new(1);
static UDP_ID: AtomicU64 = AtomicU64::new(1);


pub async fn run_tcp(addr: SocketAddr, tx: UnboundedSender<Command>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("TCP listening on {}", addr);
    loop {
        let (mut socket, peer) = listener.accept().await?;
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => break, // closed
                    Ok(n) => {
                        let id = TCP_ID.fetch_add(1, Ordering::Relaxed);
                        let payload = buf[..n].to_vec();
                        let _ = tx.send(Command::TcpPacket { id, peer, payload: payload.clone() });
                        // echo back
                        if socket.write_all(&payload).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("tcp read err={} peer={}", e, peer);
                        break;
                    }
                }
            }
        });
    }
}