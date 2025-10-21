use crate::common::Command;
use axum::{Router, extract::Path, routing::get};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc::UnboundedSender;

static REQ_ID: AtomicU64 = AtomicU64::new(1);

pub async fn run(addr: SocketAddr) -> anyhow::Result<()> {
    let app = Router::new()
        .route(
            "/{*path}",
            get(move |Path(path): Path<String>| {
                // let tx = tx.clone();
                async move {
                    // let id = REQ_ID.fetch_add(1, Ordering::Relaxed);
                    // let cmd = Command::HttpRequest { id, path };
                    // // best-effort send
                    // let _ = tx.send(cmd);
                    "ok"
                }
            }),
        )
        .route("/hello", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
