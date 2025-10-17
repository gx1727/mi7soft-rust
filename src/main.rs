//! 主服务器应用
//! 演示模块化分层架构的使用

use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

use rust_axum_learning::{
    app::{
        app1::{handler as app1_handler, service::UserService},
        app2::{handler as app2_handler, service::ProductService},
    },
    core::middleware::request_logging_middleware,
    infrastructure::logger::Logger,
};

#[derive(Clone)]
struct AppState {
    app1_state: app1_handler::AppState,
    app2_state: app2_handler::AppState,
}

#[tokio::main]
async fn main() {
    // 初始化日志
    Logger::init(tracing::Level::INFO);

    info!("启动模块化分层架构服务器...");

    // 创建应用状态
    let state = AppState {
        app1_state: app1_handler::AppState {
            user_service: UserService::new(),
        },
        app2_state: app2_handler::AppState {
            product_service: ProductService::new(),
        },
    };

    // 创建路由
    let app1_routes = Router::new()
        .route("/users/:id", get(app1_handler::get_user))
        .route("/users", post(app1_handler::create_user))
        .with_state(state.app1_state);

    let app2_routes = Router::new()
        .route("/products/:id", get(app2_handler::get_product))
        .route("/products", post(app2_handler::create_product))
        .with_state(state.app2_state);

    let app = Router::new()
        .nest("/api/v1", app1_routes)
        .nest("/api/v2", app2_routes)
        .layer(axum::middleware::from_fn(request_logging_middleware))
        .layer(TraceLayer::new_for_http());

    // 绑定地址
    let listener = TcpListener::bind("127.0.0.1:3005")
        .await
        .expect("无法绑定到端口 3005");

    info!("🚀 模块化分层架构服务器运行在 http://127.0.0.1:3005");
    info!("📖 可用的路由:");
    info!("   GET  /api/v1/users/:id   - 获取用户");
    info!("   POST /api/v1/users       - 创建用户");
    info!("   GET  /api/v2/products/:id - 获取产品");
    info!("   POST /api/v2/products    - 创建产品");

    // 启动服务器
    axum::serve(listener, app).await.expect("服务器启动失败");
}
