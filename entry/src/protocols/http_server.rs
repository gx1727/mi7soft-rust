use crate::protocols::common::{Command, ErrorResponse, HttpRequestBody, HttpResponse};
use axum::{
    Json, Router,
    body::Body,
    extract::{Query, Request, State},
    http::{HeaderMap, Method, StatusCode},
    response::Json as ResponseJson,
    routing::any,
};
use mi7::{DefaultCrossProcessQueue, Message};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, error, info, warn};

static REQ_ID: AtomicU64 = AtomicU64::new(1);

/// HTTP 服务器状态
#[derive(Clone)]
struct AppState {
    queue: Arc<DefaultCrossProcessQueue>,
    // 免鉴权路径映射
    no_auth_paths: Arc<HashMap<String, bool>>,
}

pub async fn run(addr: SocketAddr, queue: DefaultCrossProcessQueue) -> anyhow::Result<()> {
    // 初始化免鉴权路径
    let mut no_auth_paths = HashMap::new();
    no_auth_paths.insert("/health".to_string(), true);
    no_auth_paths.insert("/status".to_string(), true);
    no_auth_paths.insert("/ping".to_string(), true);

    let state = AppState {
        queue: Arc::new(queue),
        no_auth_paths: Arc::new(no_auth_paths),
    };

    // 使用统一的处理器处理所有路由
    let app = Router::new().fallback(unified_handler).with_state(state);

    info!("HTTP 服务器启动成功，监听地址: {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// 简易鉴权函数
async fn authenticate(token: &str) -> bool {
    // 简易处理：所有鉴权都成功
    debug!("[AUTH] 开始验证 token，长度: {}", token.len());

    // 模拟鉴权处理时间
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    // 这里可以添加真实的鉴权逻辑
    // 例如：验证 JWT token、查询数据库等

    let is_valid = true; // 简易处理：始终返回成功
    debug!(
        "[AUTH] token 验证结果: {}",
        if is_valid { "成功" } else { "失败" }
    );

    is_valid
}

/// 统一的请求处理器
async fn unified_handler(
    State(state): State<AppState>,
    method: Method,
    uri: axum::http::Uri,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    request: Request<Body>,
) -> Result<ResponseJson<Value>, (StatusCode, ResponseJson<ErrorResponse>)> {
    let start_time = std::time::Instant::now();
    let task_id = REQ_ID.fetch_add(1, Ordering::Relaxed);
    let path = uri.path().to_string();
    let method_str = method.to_string();
    let query_string = uri.query().unwrap_or("");

    // 记录请求开始
    info!(
        "[REQUEST_START] 任务ID: {}, 方法: {}, 路径: {}, 查询参数: {}",
        task_id, method_str, path, query_string
    );

    // 记录请求头信息（仅记录关键头）
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");
    let content_type = headers
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("none");

    debug!(
        "[REQUEST_DETAILS] 任务ID: {}, User-Agent: {}, Content-Type: {}, 参数数量: {}",
        task_id,
        user_agent,
        content_type,
        params.len()
    );

    if !params.is_empty() {
        debug!("[REQUEST_PARAMS] 任务ID: {}, 参数: {:?}", task_id, params);
    }

    // 1. 读取 method 和 url（已完成）

    // 2. 读取 header 中的 authorization
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // 3. 判断是否需要鉴权
    let needs_auth = !state.no_auth_paths.contains_key(&path);

    debug!(
        "[AUTH_CHECK] 任务ID: {}, 需要鉴权: {}, 路径: {}",
        task_id, needs_auth, path
    );

    // 4. 如果需要鉴权，调用鉴权接口
    if needs_auth {
        if auth_header.is_empty() {
            warn!(
                "[AUTH_FAILED] 任务ID: {}, 原因: 缺少 authorization header",
                task_id
            );
            let elapsed = start_time.elapsed();
            error!(
                "[REQUEST_END] 任务ID: {}, 状态: 401 Unauthorized, 耗时: {:?}",
                task_id, elapsed
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                ResponseJson(ErrorResponse {
                    error: "缺少 authorization header".to_string(),
                    code: 401,
                }),
            ));
        }

        debug!(
            "[AUTH_VERIFY] 任务ID: {}, Token 长度: {}",
            task_id,
            auth_header.len()
        );

        if !authenticate(auth_header).await {
            warn!("[AUTH_FAILED] 任务ID: {}, 原因: Token 验证失败", task_id);
            let elapsed = start_time.elapsed();
            error!(
                "[REQUEST_END] 任务ID: {}, 状态: 401 Unauthorized, 耗时: {:?}",
                task_id, elapsed
            );
            return Err((
                StatusCode::UNAUTHORIZED,
                ResponseJson(ErrorResponse {
                    error: "鉴权失败".to_string(),
                    code: 401,
                }),
            ));
        }

        info!("[AUTH_SUCCESS] 任务ID: {}", task_id);
    } else {
        info!("[NO_AUTH_PATH] 任务ID: {}, 路径: {}", task_id, path);
    }

    // 5. 获取 param 或 post 数据
    let body_data = if method == Method::POST || method == Method::PUT {
        debug!("[READ_BODY] 任务ID: {}, 方法: {}", task_id, method_str);
        // 读取请求体
        let (parts, body) = request.into_parts();
        match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => {
                let body_str = String::from_utf8_lossy(&bytes).to_string();
                let body_size = bytes.len();
                debug!(
                    "[BODY_CONTENT] 任务ID: {}, 大小: {} bytes",
                    task_id, body_size
                );
                if body_size > 0 && body_size <= 1000 {
                    debug!("[BODY_DATA] 任务ID: {}, 内容: {}", task_id, body_str);
                } else if body_size > 1000 {
                    debug!(
                        "[BODY_DATA] 任务ID: {}, 内容过长，仅显示前100字符: {}",
                        task_id,
                        &body_str[..100.min(body_str.len())]
                    );
                }
                Some(body_str)
            }
            Err(e) => {
                error!("[BODY_READ_ERROR] 任务ID: {}, 错误: {}", task_id, e);
                let elapsed = start_time.elapsed();
                error!(
                    "[REQUEST_END] 任务ID: {}, 状态: 400 Bad Request, 耗时: {:?}",
                    task_id, elapsed
                );
                return Err((
                    StatusCode::BAD_REQUEST,
                    ResponseJson(ErrorResponse {
                        error: format!("读取请求体失败: {}", e),
                        code: 400,
                    }),
                ));
            }
        }
    } else {
        debug!("[NO_BODY] 任务ID: {}, 方法: {}", task_id, method_str);
        None
    };

    // 创建命令
    let cmd = Command::HttpRequest {
        id: task_id,
        path: path.clone(),
        method: method_str.clone(),
        body: body_data.clone(),
        headers: if params.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&params).unwrap_or_default())
        },
    };

    debug!(
        "[CREATE_CMD] 任务ID: {}, 路径: {}, 方法: {}, 有请求体: {}",
        task_id,
        path,
        method_str,
        body_data.is_some()
    );

    // 6. 通过共享内存发送给 worker
    debug!("[SERIALIZE] 任务ID: {}", task_id);
    let serialized = match bincode::encode_to_vec(&cmd, bincode::config::standard()) {
        Ok(data) => {
            debug!(
                "[SERIALIZE_OK] 任务ID: {}, 数据大小: {} bytes",
                task_id,
                data.len()
            );
            data
        }
        Err(e) => {
            error!("[SERIALIZE_ERROR] 任务ID: {}, 错误: {}", task_id, e);
            let elapsed = start_time.elapsed();
            error!(
                "[REQUEST_END] 任务ID: {}, 状态: 500 Internal Server Error, 耗时: {:?}",
                task_id, elapsed
            );
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

    debug!(
        "[SEND_QUEUE] 任务ID: {}, 消息大小: {} bytes",
        task_id,
        message.data.len()
    );

    match state.queue.send(message) {
        Ok(_) => {
            let elapsed = start_time.elapsed();
            info!(
                "[QUEUE_SEND_OK] 任务ID: {}, 方法: {}, 路径: {}, 耗时: {:?}",
                task_id, method_str, path, elapsed
            );

            // 获取队列状态用于日志
            let queue_status = state.queue.status();
            debug!(
                "[QUEUE_STATUS] 任务ID: {}, 当前消息数: {}/{}",
                task_id, queue_status.message_count, queue_status.capacity
            );

            // 特殊处理状态查询
            if path == "/status" {
                let response = serde_json::json!({
                    "server": "Mi7Soft HTTP Server",
                    "status": "running",
                    "task_id": task_id,
                    "queue": {
                        "capacity": queue_status.capacity,
                        "current_size": queue_status.message_count,
                        "status": "connected"
                    }
                });
                info!(
                    "[STATUS_RESPONSE] 任务ID: {}, 队列: {}/{}",
                    task_id, queue_status.message_count, queue_status.capacity
                );
                Ok(ResponseJson(response))
            } else {
                let response = serde_json::json!({
                    "success": true,
                    "message": format!("{} 请求 {} 已处理", method_str, path),
                    "task_id": task_id
                });
                info!(
                    "[REQUEST_SUCCESS] 任务ID: {}, 方法: {}, 路径: {}, 总耗时: {:?}",
                    task_id, method_str, path, elapsed
                );
                Ok(ResponseJson(response))
            }
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            error!(
                "[QUEUE_SEND_ERROR] 任务ID: {}, 错误: {}, 耗时: {:?}",
                task_id, e, elapsed
            );
            error!(
                "[REQUEST_END] 任务ID: {}, 状态: 500 Internal Server Error, 耗时: {:?}",
                task_id, elapsed
            );
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
