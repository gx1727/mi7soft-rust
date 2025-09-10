# Axum Web 框架完整指南

Axum 是一个现代、高性能的 Rust Web 框架，专为异步编程设计。本指南将带你深入了解 Axum 的核心概念和最佳实践。

## 📋 目录

1. [Axum 简介](#axum-简介)
2. [核心概念](#核心概念)
3. [路由系统](#路由系统)
4. [处理器函数](#处理器函数)
5. [提取器 (Extractors)](#提取器-extractors)
6. [响应类型](#响应类型)
7. [中间件](#中间件)
8. [状态管理](#状态管理)
9. [错误处理](#错误处理)
10. [测试](#测试)

## Axum 简介

### 什么是 Axum？

Axum 是由 Tokio 团队开发的 Web 框架，具有以下特点：

- **类型安全**：编译时检查，减少运行时错误
- **高性能**：基于 Tokio 异步运行时
- **模块化**：基于 Tower 中间件生态系统
- **人体工程学**：简洁的 API 设计
- **可扩展**：丰富的中间件支持

### 基本依赖

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

## 核心概念

### 应用程序结构

```rust
use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // 创建路由
    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user));
    
    // 启动服务器
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_user() -> &'static str {
    "User created"
}
```

### 异步处理器

Axum 的所有处理器都是异步函数：

```rust
// 简单异步处理器
async fn hello() -> &'static str {
    "Hello, World!"
}

// 带异步操作的处理器
async fn fetch_data() -> String {
    // 模拟异步数据库查询
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    "Data from database".to_string()
}
```

## 路由系统

### 基本路由

```rust
use axum::{
    routing::{get, post, put, delete, patch},
    Router,
};

let app = Router::new()
    // HTTP 方法路由
    .route("/", get(root))
    .route("/users", post(create_user))
    .route("/users/:id", get(get_user))
    .route("/users/:id", put(update_user))
    .route("/users/:id", delete(delete_user))
    
    // 多方法路由
    .route("/items", get(list_items).post(create_item))
    
    // 通配符路由
    .route("/files/*path", get(serve_file));
```

### 路径参数

```rust
use axum::{
    extract::Path,
    response::Json,
};
use serde::Deserialize;

// 单个参数
async fn get_user(Path(user_id): Path<u32>) -> String {
    format!("User ID: {}", user_id)
}

// 多个参数
#[derive(Deserialize)]
struct Params {
    user_id: u32,
    post_id: u32,
}

async fn get_user_post(Path(params): Path<Params>) -> String {
    format!("User: {}, Post: {}", params.user_id, params.post_id)
}

// 路由定义
let app = Router::new()
    .route("/users/:user_id", get(get_user))
    .route("/users/:user_id/posts/:post_id", get(get_user_post));
```

### 查询参数

```rust
use axum::extract::Query;
use serde::Deserialize;

#[derive(Deserialize)]
struct Pagination {
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn list_users(Query(params): Query<Pagination>) -> String {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(10);
    format!("Page: {}, Per page: {}", page, per_page)
}

// 使用: GET /users?page=2&per_page=20
```

### 嵌套路由

```rust
// API v1 路由
fn api_v1_routes() -> Router {
    Router::new()
        .route("/users", get(list_users_v1))
        .route("/posts", get(list_posts_v1))
}

// API v2 路由
fn api_v2_routes() -> Router {
    Router::new()
        .route("/users", get(list_users_v2))
        .route("/posts", get(list_posts_v2))
}

// 主应用
let app = Router::new()
    .nest("/api/v1", api_v1_routes())
    .nest("/api/v2", api_v2_routes())
    .route("/health", get(health_check));
```

## 处理器函数

### 处理器签名

Axum 处理器可以有多种签名：

```rust
// 无参数
async fn handler1() -> &'static str {
    "Hello"
}

// 带提取器
async fn handler2(Path(id): Path<u32>) -> String {
    format!("ID: {}", id)
}

// 多个提取器
async fn handler3(
    Path(id): Path<u32>,
    Query(params): Query<HashMap<String, String>>,
) -> String {
    format!("ID: {}, Params: {:?}", id, params)
}

// 带状态
async fn handler4(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<User>, StatusCode> {
    // 使用状态处理请求
    Ok(Json(User { id, name: "John".to_string() }))
}
```

### 处理器返回类型

```rust
use axum::{
    response::{Html, Json, Response},
    http::StatusCode,
};

// 字符串响应
async fn text_response() -> &'static str {
    "Plain text"
}

// HTML 响应
async fn html_response() -> Html<&'static str> {
    Html("<h1>Hello, HTML!</h1>")
}

// JSON 响应
async fn json_response() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "Hello, JSON!",
        "status": "success"
    }))
}

// 状态码 + JSON
async fn created_response() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::CREATED,
        Json(serde_json::json!({"message": "Created"})),
    )
}

// Result 类型
async fn fallible_handler() -> Result<Json<User>, StatusCode> {
    let user = fetch_user().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(user))
}
```

## 提取器 (Extractors)

### 内置提取器

```rust
use axum::{
    extract::{
        Path, Query, Json, Form, State,
        Request, ConnectInfo,
    },
    http::{HeaderMap, Method, Uri},
};

// 路径参数
async fn path_extractor(Path(id): Path<u32>) {}

// 查询参数
async fn query_extractor(Query(params): Query<HashMap<String, String>>) {}

// JSON 请求体
async fn json_extractor(Json(payload): Json<CreateUser>) {}

// 表单数据
async fn form_extractor(Form(input): Form<LoginForm>) {}

// 请求头
async fn headers_extractor(headers: HeaderMap) {
    if let Some(user_agent) = headers.get("user-agent") {
        println!("User-Agent: {:?}", user_agent);
    }
}

// HTTP 方法和 URI
async fn method_uri_extractor(method: Method, uri: Uri) {
    println!("{} {}", method, uri);
}

// 原始请求
async fn request_extractor(request: Request) {
    // 完全控制请求
}

// 连接信息
async fn connect_info_extractor(
    ConnectInfo(addr): ConnectInfo<SocketAddr>
) {
    println!("Client address: {}", addr);
}
```

### 自定义提取器

```rust
use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};

// 自定义提取器：验证 API Key
struct ApiKey(String);

#[async_trait]
impl<S> FromRequest<S> for ApiKey
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers();
        
        if let Some(api_key) = headers.get("x-api-key") {
            if let Ok(api_key_str) = api_key.to_str() {
                return Ok(ApiKey(api_key_str.to_string()));
            }
        }
        
        Err((StatusCode::UNAUTHORIZED, "Missing or invalid API key").into_response())
    }
}

// 使用自定义提取器
async fn protected_handler(ApiKey(key): ApiKey) -> String {
    format!("Authenticated with key: {}", key)
}
```

## 响应类型

### 实现 IntoResponse

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: String,
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let status = if self.success {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        };
        
        (status, Json(self)).into_response()
    }
}

// 使用自定义响应
async fn api_handler() -> ApiResponse<User> {
    ApiResponse {
        success: true,
        data: Some(User { id: 1, name: "John".to_string() }),
        message: "User retrieved successfully".to_string(),
    }
}
```

### 流式响应

```rust
use axum::{
    response::sse::{Event, Sse},
    response::Stream,
};
use futures::stream::{self, Stream};
use std::time::Duration;

// Server-Sent Events
async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(|| {
        Event::default().data(format!("Current time: {}", chrono::Utc::now()))
    })
    .map(Ok)
    .throttle(Duration::from_secs(1));
    
    Sse::new(stream)
}

// 文件下载
use tokio_util::io::ReaderStream;

async fn download_file() -> Result<Response, StatusCode> {
    let file = tokio::fs::File::open("large_file.txt").await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    
    Ok(Response::builder()
        .header("content-type", "application/octet-stream")
        .header("content-disposition", "attachment; filename=\"file.txt\"")
        .body(body)
        .unwrap())
}
```

## 中间件

### 使用内置中间件

```rust
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    timeout::TimeoutLayer,
    compression::CompressionLayer,
};
use std::time::Duration;

let app = Router::new()
    .route("/", get(root))
    // 压缩响应
    .layer(CompressionLayer::new())
    // 请求超时
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    // CORS
    .layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
    // 请求追踪
    .layer(TraceLayer::new_for_http());
```

### 自定义中间件

```rust
use axum::{
    middleware::{self, Next},
    extract::Request,
    response::Response,
};

// 请求日志中间件
async fn logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed();
    println!(
        "{} {} - {} - {:?}",
        method,
        uri,
        response.status(),
        duration
    );
    
    response
}

// 认证中间件
async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request.headers()
        .get("authorization")
        .and_then(|header| header.to_str().ok());
    
    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            let response = next.run(request).await;
            return Ok(response);
        }
    }
    
    Err(StatusCode::UNAUTHORIZED)
}

// 应用中间件
let app = Router::new()
    .route("/public", get(public_handler))
    .route("/protected", get(protected_handler))
    .layer(middleware::from_fn(logging_middleware))
    .route_layer(middleware::from_fn(auth_middleware)); // 只对特定路由
```

## 状态管理

### 应用状态

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// 定义应用状态
#[derive(Clone)]
struct AppState {
    db: Arc<Database>,
    cache: Arc<RwLock<HashMap<String, String>>>,
    config: Config,
}

#[derive(Clone)]
struct Config {
    api_key: String,
    max_connections: u32,
}

// 在处理器中使用状态
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<User>, StatusCode> {
    // 使用数据库
    let user = state.db.get_user(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 使用缓存
    let cache_key = format!("user:{}", id);
    let mut cache = state.cache.write().await;
    cache.insert(cache_key, serde_json::to_string(&user).unwrap());
    
    Ok(Json(user))
}

// 创建应用
#[tokio::main]
async fn main() {
    let state = AppState {
        db: Arc::new(Database::new().await),
        cache: Arc::new(RwLock::new(HashMap::new())),
        config: Config {
            api_key: "secret".to_string(),
            max_connections: 100,
        },
    };
    
    let app = Router::new()
        .route("/users/:id", get(get_user))
        .with_state(state);
    
    // 启动服务器...
}
```

### 状态提取器

```rust
// 从状态中提取特定部分
#[derive(Clone, FromRef)]
struct AppState {
    database: Database,
    redis: Redis,
}

// 可以直接提取 Database
async fn handler(State(database): State<Database>) {
    // 使用 database
}

// 或者提取整个状态
async fn handler2(State(state): State<AppState>) {
    // 使用 state.database 和 state.redis
}
```

## 错误处理

### 自定义错误类型

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde_json::json;

#[derive(Debug)]
enum AppError {
    NotFound,
    Unauthorized,
    ValidationError(String),
    DatabaseError(String),
    InternalError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                "Resource not found",
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Unauthorized access",
            ),
            AppError::ValidationError(msg) => (
                StatusCode::BAD_REQUEST,
                &msg,
            ),
            AppError::DatabaseError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error occurred",
            ),
            AppError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
            ),
        };
        
        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));
        
        (status, body).into_response()
    }
}

// 在处理器中使用
async fn get_user(Path(id): Path<u32>) -> Result<Json<User>, AppError> {
    if id == 0 {
        return Err(AppError::ValidationError("ID cannot be zero".to_string()));
    }
    
    let user = database::get_user(id).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    user.ok_or(AppError::NotFound)
        .map(Json)
}
```

### 全局错误处理

```rust
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;

// Panic 处理
let app = Router::new()
    .route("/", get(root))
    .layer(
        ServiceBuilder::new()
            .layer(CatchPanicLayer::custom(handle_panic))
    );

fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic occurred".to_string()
    };
    
    eprintln!("Panic occurred: {}", details);
    
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
}
```

## 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`
    
    #[tokio::test]
    async fn test_hello_world() {
        let app = Router::new().route("/", get(|| async { "Hello, World!" }));
        
        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"Hello, World!");
    }
    
    #[tokio::test]
    async fn test_json_response() {
        let app = Router::new().route(
            "/json",
            get(|| async {
                Json(serde_json::json!({
                    "message": "Hello, JSON!"
                }))
            }),
        );
        
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }
}
```

### 集成测试

```rust
use axum_test::TestServer;

#[tokio::test]
async fn test_api_integration() {
    let app = create_app().await; // 你的应用创建函数
    let server = TestServer::new(app).unwrap();
    
    // 测试 GET 请求
    let response = server.get("/users").await;
    response.assert_status_ok();
    
    // 测试 POST 请求
    let user = serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com"
    });
    
    let response = server.post("/users")
        .json(&user)
        .await;
    response.assert_status(StatusCode::CREATED);
    
    // 验证响应内容
    let created_user: User = response.json();
    assert_eq!(created_user.name, "John Doe");
}
```

## 🎯 最佳实践

### 1. 项目结构

```
src/
├── main.rs              # 应用入口
├── lib.rs               # 库入口
├── routes/              # 路由模块
│   ├── mod.rs
│   ├── users.rs
│   └── posts.rs
├── handlers/            # 处理器
│   ├── mod.rs
│   ├── user_handlers.rs
│   └── post_handlers.rs
├── middleware/          # 自定义中间件
│   ├── mod.rs
│   ├── auth.rs
│   └── logging.rs
├── models/              # 数据模型
│   ├── mod.rs
│   ├── user.rs
│   └── post.rs
├── services/            # 业务逻辑
│   ├── mod.rs
│   ├── user_service.rs
│   └── post_service.rs
├── database/            # 数据库相关
│   ├── mod.rs
│   └── connection.rs
└── config/              # 配置
    ├── mod.rs
    └── settings.rs
```

### 2. 错误处理策略

- 使用自定义错误类型
- 实现 `IntoResponse` trait
- 提供有意义的错误信息
- 记录详细的错误日志

### 3. 性能优化

- 使用连接池
- 实现适当的缓存
- 使用流式响应处理大文件
- 启用压缩中间件

### 4. 安全考虑

- 验证所有输入
- 使用 HTTPS
- 实现适当的认证和授权
- 防止 SQL 注入和 XSS 攻击

## 📚 进阶主题

- **WebSocket 支持**：实时通信
- **GraphQL 集成**：使用 async-graphql
- **数据库集成**：SQLx、Diesel、SeaORM
- **缓存策略**：Redis、内存缓存
- **监控和指标**：Prometheus、Jaeger
- **部署策略**：Docker、Kubernetes

---

这份指南涵盖了 Axum 的核心概念和实用技巧。通过实践这些示例，你将能够构建高性能、类型安全的 Web 应用程序。记住，Axum 的强大之处在于其类型系统和异步特性的完美结合！