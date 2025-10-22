use crate::protocols::common::{Command, HttpRequestBody, HttpResponse, ErrorResponse};
use axum::{
    Router, 
    extract::{Path, State}, 
    routing::{get, post}, 
    Json,
    http::StatusCode,
    response::Json as ResponseJson,
};
use mi7::{CrossProcessQueue, Message};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{info, error, warn};

static REQ_ID: AtomicU64 = AtomicU64::new(1);

/// HTTP 服务器状态
#[derive(Clone)]
struct AppState {
    queue: Arc<CrossProcessQueue>,
}

pub async fn run(addr: SocketAddr, queue: CrossProcessQueue) -> anyhow::Result<()> {
    let state = AppState {
        queue: Arc::new(queue),
    };

    let app = Router::new()
        .route("/hello", get(hello_handler))
        .route("/send", post(send_message_handler))
        .route("/status", get(status_handler))
        .route(
            "/{*path}",
            get(catch_all_handler),
        )
        .with_state(state);

    info!("HTTP 服务器启动成功，监听地址: {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Hello 处理器
async fn hello_handler() -> &'static str {
    "Hello, Mi7Soft Message Queue!"
}

/// 发送消息处理器
async fn send_message_handler(
    State(state): State<AppState>,
    Json(payload): Json<HttpRequestBody>,
) -> Result<ResponseJson<HttpResponse>, (StatusCode, ResponseJson<ErrorResponse>)> {
    let task_id = REQ_ID.fetch_add(1, Ordering::Relaxed);
    
    info!("收到 HTTP 请求，任务ID: {}, 消息: {}", task_id, payload.message);

    // 创建命令
    let cmd = Command::HttpRequest {
        id: task_id,
        path: "/send".to_string(),
        method: "POST".to_string(),
        body: Some(payload.message.clone()),
        headers: payload.data.map(|d| d.to_string()),
    };

    // 序列化命令为消息
    let serialized = match bincode::encode_to_vec(&cmd, bincode::config::standard()) {
        Ok(data) => data,
        Err(e) => {
            error!("序列化命令失败，任务ID: {}, 错误: {}", task_id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(ErrorResponse {
                    error: format!("序列化失败: {}", e),
                    code: 500,
                }),
            ));
        }
    };

    let message = Message {
        id: task_id,
        data: serialized,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    // 发送到消息队列
    match state.queue.send(message) {
        Ok(_) => {
            info!("消息已发送到队列，任务ID: {}", task_id);
            Ok(ResponseJson(HttpResponse {
                success: true,
                message: "消息已成功发送到队列".to_string(),
                task_id: Some(task_id),
            }))
        }
        Err(e) => {
            error!("发送消息到队列失败，任务ID: {}, 错误: {}", task_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(ErrorResponse {
                    error: format!("发送消息失败: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}

/// 状态查询处理器
async fn status_handler(State(state): State<AppState>) -> ResponseJson<serde_json::Value> {
    let queue_status = state.queue.status();
    let queue_info = serde_json::json!({
        "queue_name": "task_queue",
        "capacity": queue_status.capacity,
        "current_size": queue_status.message_count,
        "status": "connected"
    });

    ResponseJson(serde_json::json!({
        "server": "Mi7Soft HTTP Server",
        "status": "running",
        "queue": queue_info
    }))
}

/// 通用路径处理器
async fn catch_all_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<ResponseJson<HttpResponse>, (StatusCode, ResponseJson<ErrorResponse>)> {
    let task_id = REQ_ID.fetch_add(1, Ordering::Relaxed);
    
    info!("收到 GET 请求，路径: {}, 任务ID: {}", path, task_id);

    let cmd = Command::HttpRequest {
        id: task_id,
        path: path.clone(),
        method: "GET".to_string(),
        body: None,
        headers: None,
    };

    // 序列化命令为消息
    let serialized = match bincode::encode_to_vec(&cmd, bincode::config::standard()) {
        Ok(data) => data,
        Err(e) => {
            error!("序列化命令失败，路径: {}, 任务ID: {}, 错误: {}", path, task_id, e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(ErrorResponse {
                    error: format!("序列化失败: {}", e),
                    code: 500,
                }),
            ));
        }
    };

    let message = Message {
        id: task_id,
        data: serialized,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    match state.queue.send(message) {
        Ok(_) => {
            info!("GET 请求已发送到队列，路径: {}, 任务ID: {}", path, task_id);
            Ok(ResponseJson(HttpResponse {
                success: true,
                message: format!("GET 请求 {} 已处理", path),
                task_id: Some(task_id),
            }))
        }
        Err(e) => {
            error!("发送 GET 请求到队列失败，路径: {}, 任务ID: {}, 错误: {}", path, task_id, e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(ErrorResponse {
                    error: format!("处理请求失败: {}", e),
                    code: 500,
                }),
            ))
        }
    }
}
