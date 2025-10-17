//! 基础的 Axum Web 服务器
//! 演示如何创建一个简单的 HTTP 服务器

use axum::{
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("启动 Axum 基础服务器...");

    // 创建路由
    let app = Router::new()
        .route("/", get(home_handler))
        .route("/hello", get(hello_handler))
        .route("/json", get(json_handler))
        .route("/echo", post(echo_handler))
        .route("/user/:name", get(user_handler))
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http()); // 添加请求追踪中间件

    // 绑定地址
    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("无法绑定到端口 3000");

    info!("🚀 服务器运行在 http://127.0.0.1:3000");
    info!("📖 可用的路由:");
    info!("   GET  /           - 主页");
    info!("   GET  /hello      - 问候页面");
    info!("   GET  /json       - JSON 响应");
    info!("   POST /echo       - 回显 JSON");
    info!("   GET  /user/:name - 用户信息");
    info!("   GET  /health     - 健康检查");

    // 启动服务器
    axum::serve(listener, app).await.expect("服务器启动失败");
}

/// 主页处理器
async fn home_handler() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Axum 学习服务器</title>
            <meta charset="UTF-8">
            <style>
                body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
                .container { max-width: 800px; margin: 0 auto; background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
                h1 { color: #333; text-align: center; }
                .route { background: #f8f9fa; padding: 15px; margin: 10px 0; border-radius: 5px; border-left: 4px solid #007bff; }
                .method { font-weight: bold; color: #007bff; }
                a { color: #007bff; text-decoration: none; }
                a:hover { text-decoration: underline; }
            </style>
        </head>
        <body>
            <div class="container">
                <h1>🦀 欢迎来到 Axum 学习服务器！</h1>
                <p>这是一个用 Rust 和 Axum 框架构建的 Web 服务器示例。</p>
                
                <h2>📚 可用的路由:</h2>
                
                <div class="route">
                    <span class="method">GET</span> <a href="/hello">/hello</a> - 简单的问候页面
                </div>
                
                <div class="route">
                    <span class="method">GET</span> <a href="/json">/json</a> - 返回 JSON 数据
                </div>
                
                <div class="route">
                    <span class="method">POST</span> /echo - 回显发送的 JSON 数据
                </div>
                
                <div class="route">
                    <span class="method">GET</span> <a href="/user/张三">/user/:name</a> - 获取用户信息 (例: /user/张三)
                </div>
                
                <div class="route">
                    <span class="method">GET</span> <a href="/health">/health</a> - 服务器健康检查
                </div>
                
                <h2>🛠️ 测试建议:</h2>
                <p>你可以使用以下工具测试 API:</p>
                <ul>
                    <li>浏览器 - 测试 GET 请求</li>
                    <li>curl - 命令行工具</li>
                    <li>Postman - API 测试工具</li>
                    <li>Thunder Client - VS Code 扩展</li>
                </ul>
            </div>
        </body>
        </html>
        "#,
    )
}

/// 简单问候处理器
async fn hello_handler() -> Html<&'static str> {
    Html("<h1>你好，Axum！</h1><p>这是一个简单的 HTML 响应。</p><a href='/'>返回主页</a>")
}

/// JSON 响应处理器
async fn json_handler() -> Json<serde_json::Value> {
    let mut response = HashMap::new();
    response.insert("message", "Hello from Axum!");
    response.insert("framework", "Axum");
    response.insert("language", "Rust");
    response.insert("status", "success");

    Json(serde_json::json!({
        "data": response,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "server": "Axum Learning Server"
    }))
}

/// 用户信息处理器（路径参数示例）
async fn user_handler(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "user": {
            "name": name,
            "id": uuid::Uuid::new_v4(),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "status": "active"
        },
        "message": format!("用户 {} 的信息", name)
    }))
}

/// 回显处理器（POST 请求示例）
#[derive(Deserialize, Serialize)]
struct EchoRequest {
    message: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Serialize)]
struct EchoResponse {
    echo: String,
    received_at: String,
    from: String,
    original: EchoRequest,
}

async fn echo_handler(Json(payload): Json<EchoRequest>) -> Json<EchoResponse> {
    let response = EchoResponse {
        echo: format!("收到消息: {}", payload.message),
        received_at: chrono::Utc::now().to_rfc3339(),
        from: payload
            .name
            .clone()
            .unwrap_or_else(|| "匿名用户".to_string()),
        original: payload,
    };

    Json(response)
}

/// 健康检查处理器
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uptime": "运行中",
        "version": "0.1.0",
        "framework": "Axum 0.7"
    }))
}
