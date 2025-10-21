use tracing::{debug, error, info};

use crate::common::Command;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::mpsc::UnboundedSender;
use std::sync::atomic::{AtomicU64, Ordering};


static WS_ID: AtomicU64 = AtomicU64::new(1);


pub async fn run(addr: SocketAddr, tx: UnboundedSender<Command>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("WebSocket listening on {}", addr);


    loop {
        let (stream, peer) = listener.accept().await?;
        let tx = tx.clone();
        tokio::spawn(async move {
            let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                Ok(s) => s,
                Err(e) => {
                    error!("ws accept err={} peer={}", e, peer);
                    return;
                }
            };
            let (mut write, mut read) = ws_stream.split();
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(t)) => {
                        let id = WS_ID.fetch_add(1, Ordering::Relaxed);
                        let client = peer.to_string();
                        let _ = tx.send(Command::WsMessage { id, client, payload: t.clone() });


                        // echo back simple acknowledgement
                        if write.send(Message::Text(format!("recv id={}", id))).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Binary(_)) => {}
                    Ok(Message::Ping(_)) => {}
                    Ok(Message::Close(_)) => break,
                    Err(e) => {
                        error!("ws err={} peer={}", e, peer);
                        break;
                    }
                    _ => {}
                }
            }
        });
    }
}