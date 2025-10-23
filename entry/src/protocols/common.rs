use serde::{Deserialize, Serialize};

/// 跨进程消息命令枚举
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub enum Command {
    /// HTTP 请求命令
    HttpRequest {
        id: u64,
        path: String,
        method: String,
        body: Option<String>,
        headers: Option<String>,
    },
    /// WebSocket 消息
    WsMessage {
        id: u64,
        client: String,
        payload: String,
    },
    /// TCP 数据包
    TcpPacket {
        id: u64,
        peer: std::net::SocketAddr,
        payload: Vec<u8>,
    },
    /// UDP 数据包
    UdpPacket {
        id: u64,
        peer: std::net::SocketAddr,
        payload: Vec<u8>,
    },
    /// MQTT 发布消息
    MqttPublish {
        id: u64,
        topic: String,
        payload: Vec<u8>,
    },
}

/// HTTP 请求体结构
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequestBody {
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// HTTP 响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub success: bool,
    pub message: String,
    pub task_id: Option<u64>,
}

/// 错误响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}