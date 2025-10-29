use crate::protocols::common::{Command, ErrorResponse, HttpRequestBody, HttpResponse};
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{Json as ResponseJson, IntoResponse, Response},
};
use mi7::{CrossProcessPipe, Message, shared_slot::SlotState};
use serde_json::Value;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicI64, AtomicU64, Ordering},
    },
};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

// 全局响应映射表，用于存储等待响应的 oneshot 发送端
lazy_static::lazy_static! {
    static ref REQ_ID: AtomicU64 = AtomicU64::new(1);
    static ref RESPONSE_MAP: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

/// 后台响应处理循环
pub async fn response_handler_loop() {
    info!("[RESPONSE_HANDLER] 后台响应处理循环已启动");

    loop {
        // 每秒检查一次 RESPONSE_MAP
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let mut to_remove = Vec::new();

        // 检查是否有待处理的响应
        {
            let response_map = RESPONSE_MAP.lock().unwrap();

            if !response_map.is_empty() {
                debug!(
                    "[RESPONSE_HANDLER] 检查到 {} 个待处理响应",
                    response_map.len()
                );

                // 简易处理：为每个请求生成一个模拟响应
                for (&task_id, _) in response_map.iter() {
                    to_remove.push(task_id);
                }
            }
        }

        // 处理所有待处理的响应
        for task_id in to_remove {
            let tx = {
                let mut response_map = RESPONSE_MAP.lock().unwrap();
                response_map.remove(&task_id)
            };

            if let Some(tx) = tx {
                // 生成模拟响应
                let result = serde_json::json!({
                    "success": true,
                    "message": "请求已由 worker 处理完成",
                    "task_id": task_id,
                    "processed_at": chrono::Utc::now().to_rfc3339()
                });

                // 发送响应
                match tx.send(result) {
                    Ok(_) => {
                        info!("[RESPONSE_HANDLER] 任务ID: {} 响应已发送", task_id);
                    }
                    Err(_) => {
                        warn!(
                            "[RESPONSE_HANDLER] 任务ID: {} 响应发送失败，接收端已关闭",
                            task_id
                        );
                    }
                }
            }
        }
    }
}

/// HTTP 服务器状态
#[derive(Clone)]
struct AppState {
    queue: Arc<CrossProcessPipe<100, 4096>>,
    // 不需要鉴权的路径列表
    no_auth_paths: Arc<HashMap<String, bool>>,
    // 调度者相关字段
    counter: Arc<AtomicI64>,
    slot_sender: mpsc::UnboundedSender<usize>,
}

pub async fn run(
    addr: SocketAddr,
    queue: Arc<CrossProcessPipe<100, 4096>>,
    counter: Arc<AtomicI64>,
    slot_sender: mpsc::UnboundedSender<usize>,
) -> anyhow::Result<()> {
    // 初始化免鉴权路径
    let mut no_auth_paths = HashMap::new();
    no_auth_paths.insert("/health".to_string(), true);
    no_auth_paths.insert("/status".to_string(), true);
    no_auth_paths.insert("/ping".to_string(), true);

    let state = AppState {
        queue,
        no_auth_paths: Arc::new(no_auth_paths),
        counter,
        slot_sender,
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
    request: Request<Body>,
) -> Response {
    let start_time = std::time::Instant::now();
    let task_id = REQ_ID.fetch_add(1, Ordering::Relaxed);

    // 从 request 中提取信息
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let path = uri.path().to_string();
    let method_str = method.to_string();
    let query_string = uri.query().unwrap_or("");

    // 解析查询参数
    let params: HashMap<String, String> = uri
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default();

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
            return (
                StatusCode::UNAUTHORIZED,
                ResponseJson(ErrorResponse {
                    error: "缺少 authorization header".to_string(),
                    code: 401,
                }),
            ).into_response();
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
            return (
                StatusCode::UNAUTHORIZED,
                ResponseJson(ErrorResponse {
                    error: "鉴权失败".to_string(),
                    code: 401,
                }),
            ).into_response();
        }

        info!("[AUTH_SUCCESS] 任务ID: {}", task_id);
    } else {
        info!("[NO_AUTH_PATH] 任务ID: {}, 路径: {}", task_id, path);
    }

    // 5. 获取 param 或 post 数据
    let body_data = if method == Method::POST || method == Method::PUT {
        debug!("[READ_BODY] 任务ID: {}, 方法: {}", task_id, method_str);
        // 读取请求体
        let (_parts, body) = request.into_parts();
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
                return (
                    StatusCode::BAD_REQUEST,
                    ResponseJson(ErrorResponse {
                        error: format!("读取请求体失败: {}", e),
                        code: 400,
                    }),
                ).into_response();
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                ResponseJson(ErrorResponse {
                    error: format!("序列化失败: {}", e),
                    code: 500,
                }),
            ).into_response();
        }
    };

    let message = Message {
        flag: 0,
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

    // 使用新的调度者架构
    // 1. 请求槽位 - 增加计数器信号，通知调度者需要更多槽位
    debug!("[SLOT_REQUEST] 任务ID: {}, 请求槽位", task_id);
    state.counter.fetch_add(1, Ordering::AcqRel);
    debug!(
        "[SLOT_WAIT] 任务ID: {}, 请求槽位，counter +1: {}",
        task_id,
        state.counter.load(Ordering::Acquire)
    );

    // 2. 获取空闲槽位
    let slot_index = {
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 100; // 最多重试100次
        const RETRY_DELAY_MS: u64 = 1; // 每次重试间隔1ms

        loop {
            // 尝试获取空闲槽位
            if let Ok(index) = state.queue.hold() {
                debug!(
                    "[SLOT_ACQUIRED] 任务ID: {}, 槽位: {}, 状态: WRITING",
                    task_id, index
                );
                break index;
            } else {
                retry_count += 1;
                if retry_count >= MAX_RETRIES {
                    let elapsed = start_time.elapsed();
                    error!(
                        "[SLOT_TIMEOUT] 任务ID: {}, 等待槽位超时，重试次数: {}, 耗时: {:?}",
                        task_id, retry_count, elapsed
                    );
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        ResponseJson(ErrorResponse {
                            error: "服务器繁忙，等待槽位超时".to_string(),
                            code: 503,
                        }),
                    ).into_response();
                }

                // 短暂等待后重试
                tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    };

    // 3. 写入数据到槽位
    debug!(
        "[SLOT_WRITE] 任务ID: {}, 槽位: {}, 写入数据",
        task_id, slot_index
    );
    if let Err(e) = state.queue.send(slot_index, message) {
        let elapsed = start_time.elapsed();
        error!(
            "[SLOT_WRITE_ERROR] 任务ID: {}, 写入槽位失败: {}, 耗时: {:?}",
            task_id, e, elapsed
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(ErrorResponse {
                error: "写入槽位失败".to_string(),
                code: 500,
            }),
        ).into_response();
    }

    let elapsed = start_time.elapsed();
    info!(
        "[QUEUE_SEND_OK] 任务ID: {}, 方法: {}, 路径: {}, 耗时: {:?}",
        task_id, method_str, path, elapsed
    );

    // 获取队列状态用于日志
    let queue_status = state.queue.status();
    debug!(
        "[QUEUE_STATUS] 任务ID: {}, 当前消息数: {}/{}",
        task_id, queue_status.used_count, queue_status.capacity
    );

    // 特殊处理状态查询 - 立即返回，不等待 worker 响应
    if path == "/status" {
        let response = serde_json::json!({
            "server": "Mi7Soft HTTP Server",
            "status": "running",
            "task_id": task_id,
            "queue": {
                "capacity": queue_status.capacity,
                "current_size": queue_status.used_count,
                "status": "connected"
            }
        });
        info!(
            "[STATUS_RESPONSE] 任务ID: {}, 队列: {}/{}",
            task_id, queue_status.used_count, queue_status.capacity
        );
        ResponseJson(response).into_response()
    } else {
        // 创建 oneshot 通道等待 worker 响应
        let (tx, rx) = oneshot::channel();

        // 将发送端存储到全局映射表中
        {
            let mut response_map = RESPONSE_MAP.lock().unwrap();
            response_map.insert(task_id, tx);
        }

        debug!("[ONESHOT_CREATED] 任务ID: {}, 等待 worker 响应", task_id);

        // 异步等待 worker 响应
        match rx.await {
            Ok(result) => {
                let total_elapsed = start_time.elapsed();
                info!(
                    "[REQUEST_SUCCESS] 任务ID: {}, 方法: {}, 路径: {}, 总耗时: {:?}",
                    task_id, method_str, path, total_elapsed
                );
                ResponseJson(result).into_response()
            }
            Err(_) => {
                let total_elapsed = start_time.elapsed();
                error!(
                    "[ONESHOT_TIMEOUT] 任务ID: {}, 等待响应超时, 耗时: {:?}",
                    task_id, total_elapsed
                );

                // 清理映射表中的条目
                {
                    let mut response_map = RESPONSE_MAP.lock().unwrap();
                    response_map.remove(&task_id);
                }

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ResponseJson(ErrorResponse {
                        error: "请求处理超时".to_string(),
                        code: 500,
                    }),
                ).into_response()
            }
        }
    }
}
