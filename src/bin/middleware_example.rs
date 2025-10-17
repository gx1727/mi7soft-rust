//! 中间件和错误处理示例
//! 演示如何在 Axum 中使用各种中间件和自定义错误处理

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{net::TcpListener, time::sleep};
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info, warn, Level};
use uuid::Uuid;

// 应用状态
#[derive(Clone)]
struct AppState {
    request_count: Arc<Mutex<u64>>,
    rate_limit: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
}

// 自定义错误类型
#[derive(Debug)]
enum AppError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    RateLimited,
    InternalServerError(String),
    Timeout,
}

// 错误响应结构
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    code: u16,
    timestamp: String,
    request_id: String,
}

// API 响应结构
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
    request_id: String,
    timestamp: String,
}

// 请求/响应日志结构
#[derive(Serialize)]
struct RequestLog {
    method: String,
    uri: String,
    status: u16,
    duration_ms: u64,
    user_agent: Option<String>,
    ip: String,
}

// 示例数据结构
#[derive(Serialize, Deserialize)]
struct CreateItemRequest {
    name: String,
    description: Option<String>,
}

#[derive(Serialize)]
struct Item {
    id: String,
    name: String,
    description: Option<String>,
    created_at: String,
}

// 实现 IntoResponse for AppError
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, user_message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "认证失败，请提供有效的认证信息".to_string(),
            ),
            AppError::Forbidden => (
                StatusCode::FORBIDDEN,
                "FORBIDDEN",
                "权限不足，无法访问此资源".to_string(),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            AppError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "请求过于频繁，请稍后再试".to_string(),
            ),
            AppError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
                msg,
            ),
            AppError::Timeout => (
                StatusCode::REQUEST_TIMEOUT,
                "TIMEOUT",
                "请求超时".to_string(),
            ),
        };

        let error_response = ErrorResponse {
            error: error_message.to_string(),
            message: user_message,
            code: status.as_u16(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: Uuid::new_v4().to_string(),
        };

        (status, Json(error_response)).into_response()
    }
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("启动中间件示例服务器...");

    // 创建应用状态
    let state = AppState {
        request_count: Arc::new(Mutex::new(0)),
        rate_limit: Arc::new(Mutex::new(HashMap::new())),
    };

    // 创建路由
    let app = Router::new()
        .route("/", get(home_handler))
        .route("/items", post(create_item).get(list_items))
        .route("/protected", get(protected_handler))
        .route("/slow", get(slow_handler))
        .route("/error", get(error_handler))
        .route("/stats", get(stats_handler))
        // 应用中间件层 (按顺序应用)
        .layer(middleware::from_fn(auth_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limiting_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            request_logging_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TimeoutLayer::new(Duration::from_secs(5)))
        .with_state(state);

    // 绑定地址
    let listener = TcpListener::bind("127.0.0.1:3002")
        .await
        .expect("无法绑定到端口 3002");

    info!("🚀 中间件示例服务器运行在 http://127.0.0.1:3002");
    info!("📖 可用端点:");
    info!("   GET  /           - 主页");
    info!("   GET  /items      - 获取项目列表");
    info!("   POST /items      - 创建新项目");
    info!("   GET  /protected  - 受保护的端点 (需要 Authorization header)");
    info!("   GET  /slow       - 慢响应端点 (测试超时)");
    info!("   GET  /error      - 错误测试端点");
    info!("   GET  /stats      - 服务器统计信息");
    info!("💡 提示:");
    info!("   - 访问 /protected 需要在 header 中添加 'Authorization: Bearer your-token'");
    info!("   - 频繁请求会触发限流 (每分钟最多 10 次)");
    info!("   - /slow 端点会延迟 3 秒响应");

    // 启动服务器
    axum::serve(listener, app).await.expect("服务器启动失败");
}

// 请求日志中间件
async fn request_logging_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // 增加请求计数
    {
        let mut count = state.request_count.lock().unwrap();
        *count += 1;
    }

    // 处理请求
    let response = next.run(req).await;
    let status = response.status();
    let duration = start.elapsed();

    // 记录请求日志
    let log = RequestLog {
        method: method.to_string(),
        uri: uri.to_string(),
        status: status.as_u16(),
        duration_ms: duration.as_millis() as u64,
        user_agent,
        ip: "127.0.0.1".to_string(), // 在实际应用中应该从请求中提取真实 IP
    };

    info!(
        "Request: {}",
        serde_json::to_string(&log).unwrap_or_default()
    );

    response
}

// 限流中间件
async fn rate_limiting_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let client_ip = "127.0.0.1".to_string(); // 在实际应用中应该从请求中提取真实 IP

    {
        let mut rate_limit = state.rate_limit.lock().unwrap();
        let now = Instant::now();

        // 清理过期的记录
        rate_limit.retain(|_, (time, _)| now.duration_since(*time) < Duration::from_secs(60));

        // 检查当前 IP 的请求次数
        let (last_time, count) = rate_limit.entry(client_ip.clone()).or_insert((now, 0));

        if now.duration_since(*last_time) < Duration::from_secs(60) {
            if *count >= 10 {
                warn!("Rate limit exceeded for IP: {}", client_ip);
                return Err(AppError::RateLimited);
            }
            *count += 1;
        } else {
            *last_time = now;
            *count = 1;
        }
    }

    Ok(next.run(req).await)
}

// 认证中间件
async fn auth_middleware(req: Request, next: Next) -> Result<Response, AppError> {
    // 只对 /protected 路径进行认证检查
    if req.uri().path() == "/protected" {
        let auth_header = req
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header[7..]; // 移除 "Bearer " 前缀
                if token.is_empty() {
                    return Err(AppError::Unauthorized);
                }
                // 在实际应用中，这里应该验证 token 的有效性
                info!(
                    "Authenticated request with token: {}...",
                    &token[..token.len().min(10)]
                );
            }
            _ => return Err(AppError::Unauthorized),
        }
    }

    Ok(next.run(req).await)
}

// 路由处理器
async fn home_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "欢迎来到中间件示例服务器！",
        "description": "这个服务器演示了各种 Axum 中间件的使用",
        "features": [
            "请求日志记录",
            "限流保护",
            "认证中间件",
            "CORS 支持",
            "超时处理",
            "错误处理"
        ],
        "endpoints": {
            "/items": "项目管理 API",
            "/protected": "需要认证的端点",
            "/slow": "慢响应测试",
            "/error": "错误处理测试",
            "/stats": "服务器统计"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn list_items() -> Result<Json<ApiResponse<Vec<Item>>>, AppError> {
    let items = vec![
        Item {
            id: Uuid::new_v4().to_string(),
            name: "示例项目 1".to_string(),
            description: Some("这是第一个示例项目".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        },
        Item {
            id: Uuid::new_v4().to_string(),
            name: "示例项目 2".to_string(),
            description: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    ];

    Ok(Json(ApiResponse {
        success: true,
        data: items,
        request_id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

async fn create_item(
    Json(payload): Json<CreateItemRequest>,
) -> Result<Json<ApiResponse<Item>>, AppError> {
    // 验证输入
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("项目名称不能为空".to_string()));
    }

    if payload.name.len() > 100 {
        return Err(AppError::BadRequest(
            "项目名称不能超过 100 个字符".to_string(),
        ));
    }

    let item = Item {
        id: Uuid::new_v4().to_string(),
        name: payload.name.trim().to_string(),
        description: payload
            .description
            .map(|d| d.trim().to_string())
            .filter(|d| !d.is_empty()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(ApiResponse {
        success: true,
        data: item,
        request_id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

async fn protected_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "恭喜！你已成功访问受保护的端点",
        "user_info": {
            "authenticated": true,
            "permissions": ["read", "write"],
            "role": "user"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn slow_handler() -> Result<Json<serde_json::Value>, AppError> {
    info!("Processing slow request...");

    // 模拟慢操作
    sleep(Duration::from_secs(3)).await;

    Ok(Json(serde_json::json!({
        "message": "慢响应处理完成",
        "processing_time": "3 seconds",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn error_handler() -> Result<Json<serde_json::Value>, AppError> {
    // 随机返回不同类型的错误用于测试
    let error_type = chrono::Utc::now().timestamp() % 4;

    match error_type {
        0 => Err(AppError::BadRequest("这是一个测试的错误请求".to_string())),
        1 => Err(AppError::NotFound("请求的资源不存在".to_string())),
        2 => Err(AppError::InternalServerError("服务器内部错误".to_string())),
        _ => Err(AppError::Forbidden),
    }
}

async fn stats_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let request_count = {
        let count = state.request_count.lock().unwrap();
        *count
    };

    let active_rate_limits = {
        let rate_limit = state.rate_limit.lock().unwrap();
        rate_limit.len()
    };

    Json(serde_json::json!({
        "server_stats": {
            "total_requests": request_count,
            "active_rate_limit_entries": active_rate_limits,
            "uptime": "运行中",
            "version": "0.1.0"
        },
        "middleware_info": {
            "request_logging": "enabled",
            "rate_limiting": "10 requests per minute per IP",
            "authentication": "Bearer token for /protected",
            "cors": "enabled for all origins",
            "timeout": "5 seconds"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
