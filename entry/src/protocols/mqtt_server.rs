use tracing::{debug, error, info};
#[cfg(feature = "mqtt")]
use rumqttd::{Broker, Config};

#[cfg(feature = "mqtt")]
pub async fn run_broker(_addr: std::net::SocketAddr, tx: tokio::sync::mpsc::UnboundedSender<crate::protocols::common::Command>) -> anyhow::Result<()> {
    // default config - listens on 1883
    let config = Config::default();
    let mut broker = Broker::new(config);
    // NOTE: rumqttd runs its own threads and does not directly expose a Rust async stream
    // of incoming messages to integrate with our tx easily. Production integration often
    // uses a plugin mechanism or listens to persisted messages. Here we just start it.
    std::thread::spawn(move || {
        broker.start().unwrap();
    });
    info!("MQTT broker started (rumqttd)");
    // keep alive
    futures::future::pending::<()>().await;
    Ok(())
}