use serde::{Deserialize, Serialize};
use std::net::SocketAddr;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    HttpRequest { id: u64, path: String },
    WsMessage { id: u64, client: String, payload: String },
    TcpPacket { id: u64, peer: SocketAddr, payload: Vec<u8> },
    UdpPacket { id: u64, peer: SocketAddr, payload: Vec<u8> },
    MqttPublish { topic: String, payload: Vec<u8> },
}