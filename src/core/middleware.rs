//! 核心中间件模块

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::info;

/// 请求日志中间件
pub async fn request_logging_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(req).await;
    let status = response.status();
    let duration = start.elapsed();

    info!(
        "{} {} - {} - {}ms - User-Agent: {:?}",
        method,
        uri,
        status,
        duration.as_millis(),
        user_agent
    );

    response
}
