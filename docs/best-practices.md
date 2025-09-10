# Rust + Axum 最佳实践指南

本文档总结了使用 Rust 和 Axum 开发 Web 应用程序的最佳实践，帮助你写出高质量、可维护的代码。

## 📋 目录

1. [项目结构](#项目结构)
2. [代码组织](#代码组织)
3. [错误处理](#错误处理)
4. [性能优化](#性能优化)
5. [安全实践](#安全实践)
6. [测试策略](#测试策略)
7. [日志和监控](#日志和监控)
8. [部署和运维](#部署和运维)

## 项目结构

### 推荐的目录结构

```
project-name/
├── Cargo.toml
├── README.md
├── .env.example
├── .gitignore
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yml
├── migrations/              # 数据库迁移
│   └── 001_initial.sql
├── tests/                   # 集成测试
│   ├── common/
│   └── api_tests.rs
└── src/
    ├── main.rs             # 应用入口
    ├── lib.rs              # 库入口
    ├── config/             # 配置管理
    │   ├── mod.rs
    │   └── settings.rs
    ├── database/           # 数据库相关
    │   ├── mod.rs
    │   ├── connection.rs
    │   └── migrations.rs
    ├── models/             # 数据模型
    │   ├── mod.rs
    │   ├── user.rs
    │   └── post.rs
    ├── services/           # 业务逻辑层
    │   ├── mod.rs
    │   ├── user_service.rs
    │   └── auth_service.rs
    ├── handlers/           # HTTP 处理器
    │   ├── mod.rs
    │   ├── user_handlers.rs
    │   └── auth_handlers.rs
    ├── middleware/         # 自定义中间件
    │   ├── mod.rs
    │   ├── auth.rs
    │   ├── cors.rs
    │   └── logging.rs
    ├── routes/             # 路由定义
    │   ├── mod.rs
    │   ├── api.rs
    │   └── health.rs
    ├── utils/              # 工具函数
    │   ├── mod.rs
    │   ├── validation.rs
    │   └── crypto.rs
    └── errors/             # 错误定义
        ├── mod.rs
        └── app_error.rs
```

### Cargo.toml 最佳实践

```toml
[package]
name = "my-web-app"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "A web application built with Rust and Axum"
license = "MIT"
repository = "https://github.com/username/my-web-app"
keywords = ["web", "api", "axum"]
categories = ["web-programming"]

[dependencies]
# Web framework
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1.0", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }

# Middleware and utilities
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "timeout", "compression"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Configuration
config = "0.14"
dotenvy = "0.15"

# Security
bcrypt = "0.15"
jsonwebtoken = "9.0"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1.0", features = ["v4", "serde"] }

# Validation
validator = { version = "0.18", features = ["derive"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

[dev-dependencies]
axum-test = "14.0"
tokio-test = "0.4"

[features]
default = []
# 可选功能特性
redis = ["dep:redis"]
metrics = ["dep:prometheus"]

# 性能优化
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
```

## 代码组织

### 模块化设计

```rust
// src/lib.rs
pub mod config;
pub mod database;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;
pub mod utils;

// 重新导出常用类型
pub use errors::AppError;
pub use config::Settings;

// 应用状态类型
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: Settings,
}

// 应用创建函数
pub async fn create_app(state: AppState) -> axum::Router {
    routes::create_routes()
        .with_state(state)
        .layer(middleware::create_middleware_stack())
}
```

### 路由组织

```rust
// src/routes/mod.rs
use axum::Router;
use crate::AppState;

mod api;
mod health;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .nest("/api/v1", api::routes())
}

// src/routes/api.rs
use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::{handlers, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/users", user_routes())
        .nest("/auth", auth_routes())
}

fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::user_handlers::list_users))
        .route("/", post(handlers::user_handlers::create_user))
        .route("/:id", get(handlers::user_handlers::get_user))
        .route("/:id", put(handlers::user_handlers::update_user))
        .route("/:id", delete(handlers::user_handlers::delete_user))
}

fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(handlers::auth_handlers::login))
        .route("/register", post(handlers::auth_handlers::register))
        .route("/refresh", post(handlers::auth_handlers::refresh_token))
}
```

### 处理器组织

```rust
// src/handlers/user_handlers.rs
use axum::{
    extract::{Path, Query, State},
    response::Json,
    http::StatusCode,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    AppState, AppError,
    models::{User, CreateUserRequest, UpdateUserRequest},
    services::user_service,
};

#[derive(Deserialize)]
pub struct ListUsersQuery {
    page: Option<u32>,
    limit: Option<u32>,
}

pub async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<Vec<User>>, AppError> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10).min(100); // 限制最大值
    
    let users = user_service::list_users(&state.db, page, limit).await?;
    Ok(Json(users))
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), AppError> {
    // 验证输入
    payload.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;
    
    let user = user_service::create_user(&state.db, payload).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<User>, AppError> {
    let user = user_service::get_user_by_id(&state.db, id).await?
        .ok_or(AppError::NotFound("User not found".to_string()))?;
    Ok(Json(user))
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<User>, AppError> {
    payload.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;
    
    let user = user_service::update_user(&state.db, id, payload).await?;
    Ok(Json(user))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, AppError> {
    user_service::delete_user(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

## 错误处理

### 统一错误类型

```rust
// src/errors/app_error.rs
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Authentication failed")]
    Unauthorized,
    
    #[error("Permission denied")]
    Forbidden,
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, error_code) = match &self {
            AppError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                self.to_string(),
                "NOT_FOUND",
            ),
            AppError::ValidationError(_) => (
                StatusCode::BAD_REQUEST,
                self.to_string(),
                "VALIDATION_ERROR",
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required".to_string(),
                "UNAUTHORIZED",
            ),
            AppError::Forbidden => (
                StatusCode::FORBIDDEN,
                "Permission denied".to_string(),
                "FORBIDDEN",
            ),
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error occurred".to_string(),
                "DATABASE_ERROR",
            ),
            AppError::Config(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration error".to_string(),
                "CONFIG_ERROR",
            ),
            AppError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "Invalid token".to_string(),
                "INVALID_TOKEN",
            ),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
                "INTERNAL_ERROR",
            ),
        };
        
        // 记录错误日志
        match &self {
            AppError::Database(e) | AppError::Internal(e) => {
                tracing::error!(error = %e, "Server error occurred");
            }
            _ => {
                tracing::warn!(error = %self, "Client error occurred");
            }
        }
        
        let body = Json(json!({
            "error": {
                "message": error_message,
                "code": error_code,
                "status": status.as_u16()
            }
        }));
        
        (status, body).into_response()
    }
}

// 便利的 Result 类型别名
pub type AppResult<T> = Result<T, AppError>;
```

### 错误处理最佳实践

```rust
// 1. 使用 ? 操作符进行错误传播
pub async fn get_user_posts(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> AppResult<Vec<Post>> {
    // 检查用户是否存在
    let _user = get_user_by_id(db, user_id).await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    
    // 获取用户的帖子
    let posts = sqlx::query_as!(Post, "SELECT * FROM posts WHERE user_id = $1", user_id)
        .fetch_all(db)
        .await?;
    
    Ok(posts)
}

// 2. 提供上下文信息
use anyhow::Context;

pub async fn process_payment(amount: i64) -> AppResult<()> {
    external_payment_api(amount)
        .await
        .with_context(|| format!("Failed to process payment of ${}", amount))
        .map_err(AppError::Internal)?;
    
    Ok(())
}

// 3. 自定义验证错误
use validator::{Validate, ValidationErrors};

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        let error_messages: Vec<String> = errors
            .field_errors()
            .into_iter()
            .map(|(field, errors)| {
                let messages: Vec<String> = errors
                    .iter()
                    .map(|error| {
                        error.message
                            .as_ref()
                            .map(|msg| msg.to_string())
                            .unwrap_or_else(|| format!("Invalid field: {}", field))
                    })
                    .collect();
                messages.join(", ")
            })
            .collect();
        
        AppError::ValidationError(error_messages.join("; "))
    }
}
```

## 性能优化

### 数据库优化

```rust
// 1. 使用连接池
use sqlx::postgres::PgPoolOptions;

pub async fn create_db_pool(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)                    // 最大连接数
        .min_connections(5)                     // 最小连接数
        .acquire_timeout(Duration::from_secs(8)) // 获取连接超时
        .idle_timeout(Duration::from_secs(8))    // 空闲连接超时
        .max_lifetime(Duration::from_secs(8))    // 连接最大生命周期
        .connect(database_url)
        .await
}

// 2. 使用预编译语句
pub async fn get_users_by_status(
    db: &sqlx::PgPool,
    status: &str,
    limit: i64,
) -> AppResult<Vec<User>> {
    let users = sqlx::query_as!(
        User,
        "SELECT id, name, email, status, created_at FROM users WHERE status = $1 LIMIT $2",
        status,
        limit
    )
    .fetch_all(db)
    .await?;
    
    Ok(users)
}

// 3. 批量操作
pub async fn create_users_batch(
    db: &sqlx::PgPool,
    users: Vec<CreateUserRequest>,
) -> AppResult<Vec<User>> {
    let mut tx = db.begin().await?;
    let mut created_users = Vec::new();
    
    for user in users {
        let created_user = sqlx::query_as!(
            User,
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *",
            user.name,
            user.email
        )
        .fetch_one(&mut *tx)
        .await?;
        
        created_users.push(created_user);
    }
    
    tx.commit().await?;
    Ok(created_users)
}
```

### 缓存策略

```rust
// 使用 Redis 缓存
use redis::AsyncCommands;
use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct CacheService {
    redis: redis::Client,
}

impl CacheService {
    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let redis = redis::Client::open(redis_url)?;
        Ok(Self { redis })
    }
    
    pub async fn get<T>(&self, key: &str) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut conn = self.redis.get_async_connection().await
            .map_err(|e| AppError::Internal(e.into()))?;
        
        let value: Option<String> = conn.get(key).await
            .map_err(|e| AppError::Internal(e.into()))?;
        
        match value {
            Some(json_str) => {
                let data = serde_json::from_str(&json_str)
                    .map_err(|e| AppError::Internal(e.into()))?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }
    
    pub async fn set<T>(&self, key: &str, value: &T, ttl: u64) -> AppResult<()>
    where
        T: Serialize,
    {
        let mut conn = self.redis.get_async_connection().await
            .map_err(|e| AppError::Internal(e.into()))?;
        
        let json_str = serde_json::to_string(value)
            .map_err(|e| AppError::Internal(e.into()))?;
        
        conn.set_ex(key, json_str, ttl).await
            .map_err(|e| AppError::Internal(e.into()))?;
        
        Ok(())
    }
}

// 在服务层使用缓存
pub async fn get_user_with_cache(
    db: &sqlx::PgPool,
    cache: &CacheService,
    user_id: uuid::Uuid,
) -> AppResult<User> {
    let cache_key = format!("user:{}", user_id);
    
    // 尝试从缓存获取
    if let Some(user) = cache.get::<User>(&cache_key).await? {
        return Ok(user);
    }
    
    // 从数据库获取
    let user = get_user_by_id(db, user_id).await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    
    // 存入缓存（TTL: 1小时）
    cache.set(&cache_key, &user, 3600).await?;
    
    Ok(user)
}
```

### 响应优化

```rust
// 1. 使用压缩中间件
use tower_http::compression::CompressionLayer;

let app = Router::new()
    .route("/api/data", get(get_large_data))
    .layer(CompressionLayer::new());

// 2. 流式响应
use axum::response::sse::{Event, Sse};
use futures::stream::{self, Stream};

pub async fn stream_data() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::iter(0..1000)
        .map(|i| {
            Event::default()
                .data(format!("Data chunk: {}", i))
        })
        .map(Ok)
        .throttle(Duration::from_millis(100));
    
    Sse::new(stream)
}

// 3. 分页响应
#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
}

#[derive(Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

pub async fn list_users_paginated(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<PaginatedResponse<User>>> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(10).min(100);
    let offset = (page - 1) * per_page;
    
    let (users, total) = tokio::try_join!(
        get_users_paginated(&state.db, offset, per_page),
        count_users(&state.db)
    )?;
    
    let total_pages = (total as f64 / per_page as f64).ceil() as u32;
    
    Ok(Json(PaginatedResponse {
        data: users,
        pagination: PaginationInfo {
            page,
            per_page,
            total,
            total_pages,
        },
    }))
}
```

## 安全实践

### 认证和授权

```rust
// JWT 认证中间件
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // 用户ID
    pub exp: usize,         // 过期时间
    pub iat: usize,         // 签发时间
    pub roles: Vec<String>, // 用户角色
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }
    
    pub fn create_token(&self, user_id: &str, roles: Vec<String>) -> AppResult<String> {
        let now = chrono::Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
            iat: now.timestamp() as usize,
            roles,
        };
        
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(AppError::Jwt)
    }
    
    pub fn verify_token(&self, token: &str) -> AppResult<Claims> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(AppError::Jwt)
    }
}

// 认证中间件
pub async fn auth_middleware(
    State(jwt_service): State<JwtService>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request.headers()
        .get("authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    
    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized);
    }
    
    let token = &auth_header[7..];
    let claims = jwt_service.verify_token(token)?;
    
    // 将用户信息添加到请求扩展中
    request.extensions_mut().insert(claims);
    
    Ok(next.run(request).await)
}

// 角色检查中间件
pub fn require_role(required_role: &'static str) -> impl Fn(Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, AppError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        Box::pin(async move {
            let claims = request.extensions()
                .get::<Claims>()
                .ok_or(AppError::Unauthorized)?;
            
            if !claims.roles.contains(&required_role.to_string()) {
                return Err(AppError::Forbidden);
            }
            
            Ok(next.run(request).await)
        })
    }
}
```

### 输入验证

```rust
// 使用 validator 进行数据验证
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 2, max = 50, message = "Name must be between 2 and 50 characters"))]
    pub name: String,
    
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    #[validate(custom(function = "validate_password_strength"))]
    pub password: String,
    
    #[validate(range(min = 18, max = 120, message = "Age must be between 18 and 120"))]
    pub age: Option<u8>,
}

fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_digit(10));
    let has_special = password.chars().any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));
    
    if has_uppercase && has_lowercase && has_digit && has_special {
        Ok(())
    } else {
        Err(ValidationError::new("Password must contain uppercase, lowercase, digit, and special character"))
    }
}

// 自定义验证提取器
use axum::{
    async_trait,
    extract::{FromRequest, Request},
};

pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;
    
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await
            .map_err(|_| AppError::ValidationError("Invalid JSON".to_string()))?;
        
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

// 在处理器中使用
pub async fn create_user(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<User>)> {
    // payload 已经通过验证
    let user = user_service::create_user(&state.db, payload).await?;
    Ok((StatusCode::CREATED, Json(user)))
}
```

### 安全头和 CORS

```rust
use tower_http::{
    cors::{CorsLayer, Any},
    set_header::SetResponseHeaderLayer,
};
use axum::http::{HeaderValue, header};

pub fn create_security_middleware() -> tower::layer::util::Stack<
    SetResponseHeaderLayer<HeaderValue>,
    tower::layer::util::Stack<
        SetResponseHeaderLayer<HeaderValue>,
        tower::layer::util::Stack<
            SetResponseHeaderLayer<HeaderValue>,
            CorsLayer,
        >,
    >,
> {
    // CORS 配置
    let cors = CorsLayer::new()
        .allow_origin("https://yourdomain.com".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .max_age(Duration::from_secs(3600));
    
    // 安全头
    cors
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
}
```

## 测试策略

### 单元测试

```rust
// src/services/user_service.rs
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use uuid::Uuid;
    
    async fn setup_test_db() -> PgPool {
        // 设置测试数据库
        let database_url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must be set");
        
        let pool = PgPool::connect(&database_url).await.unwrap();
        
        // 运行迁移
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        
        pool
    }
    
    #[tokio::test]
    async fn test_create_user() {
        let db = setup_test_db().await;
        
        let request = CreateUserRequest {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePass123!".to_string(),
            age: Some(25),
        };
        
        let user = create_user(&db, request).await.unwrap();
        
        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
        assert!(user.id != Uuid::nil());
        
        // 清理
        sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
            .execute(&db)
            .await
            .unwrap();
    }
    
    #[tokio::test]
    async fn test_get_nonexistent_user() {
        let db = setup_test_db().await;
        let user_id = Uuid::new_v4();
        
        let result = get_user_by_id(&db, user_id).await.unwrap();
        assert!(result.is_none());
    }
}
```

### 集成测试

```rust
// tests/api_tests.rs
use axum_test::TestServer;
use serde_json::json;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_user_crud_operations() {
    let app = common::create_test_app().await;
    let server = TestServer::new(app).unwrap();
    
    // 创建用户
    let create_request = json!({
        "name": "Test User",
        "email": "test@example.com",
        "password": "SecurePass123!",
        "age": 25
    });
    
    let response = server
        .post("/api/v1/users")
        .json(&create_request)
        .await;
    
    response.assert_status_created();
    let created_user: serde_json::Value = response.json();
    let user_id = created_user["id"].as_str().unwrap();
    
    // 获取用户
    let response = server
        .get(&format!("/api/v1/users/{}", user_id))
        .await;
    
    response.assert_status_ok();
    let user: serde_json::Value = response.json();
    assert_eq!(user["name"], "Test User");
    
    // 更新用户
    let update_request = json!({
        "name": "Updated User",
        "email": "updated@example.com"
    });
    
    let response = server
        .put(&format!("/api/v1/users/{}", user_id))
        .json(&update_request)
        .await;
    
    response.assert_status_ok();
    
    // 删除用户
    let response = server
        .delete(&format!("/api/v1/users/{}", user_id))
        .await;
    
    response.assert_status(StatusCode::NO_CONTENT);
    
    // 验证用户已删除
    let response = server
        .get(&format!("/api/v1/users/{}", user_id))
        .await;
    
    response.assert_status_not_found();
}

#[tokio::test]
async fn test_authentication_required() {
    let app = common::create_test_app().await;
    let server = TestServer::new(app).unwrap();
    
    let response = server
        .get("/api/v1/protected")
        .await;
    
    response.assert_status_unauthorized();
}

#[tokio::test]
async fn test_validation_errors() {
    let app = common::create_test_app().await;
    let server = TestServer::new(app).unwrap();
    
    let invalid_request = json!({
        "name": "",  // 太短
        "email": "invalid-email",  // 无效邮箱
        "password": "123"  // 太短
    });
    
    let response = server
        .post("/api/v1/users")
        .json(&invalid_request)
        .await;
    
    response.assert_status_bad_request();
    let error: serde_json::Value = response.json();
    assert!(error["error"]["message"].as_str().unwrap().contains("validation"));
}
```

## 日志和监控

### 结构化日志

```rust
// src/main.rs
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,axum::rejection=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}

#[tokio::main]
async fn main() {
    init_tracing();
    
    tracing::info!("Starting application");
    
    // 应用启动逻辑...
}

// 在处理器中使用结构化日志
use tracing::{info, warn, error, instrument};

#[instrument(skip(db), fields(user_id = %user_id))]
pub async fn get_user_by_id(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> AppResult<Option<User>> {
    info!("Fetching user from database");
    
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            error!(error = %e, "Database query failed");
            AppError::Database(e)
        })?;
    
    match &user {
        Some(_) => info!("User found"),
        None => warn!("User not found"),
    }
    
    Ok(user)
}
```

### 健康检查

```rust
// src/routes/health.rs
use axum::{
    routing::get,
    Router, Json,
};
use serde_json::json;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/live", get(liveness_check))
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn readiness_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 检查数据库连接
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Database health check failed");
            AppError::Database(e)
        })?;
    
    Ok(Json(json!({
        "status": "ready",
        "checks": {
            "database": "ok"
        }
    })))
}

pub async fn liveness_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "alive"
    }))
}
```

## 部署和运维

### Docker 配置

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# 构建应用
RUN cargo build --release

# 运行时镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -r -s /bin/false appuser

WORKDIR /app

# 复制二进制文件
COPY --from=builder /app/target/release/my-web-app /app/
COPY --from=builder /app/migrations /app/migrations

# 设置权限
RUN chown -R appuser:appuser /app
USER appuser

EXPOSE 3000

CMD ["./my-web-app"]
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgres://user:password@db:5432/myapp
      - REDIS_URL=redis://redis:6379
      - RUST_LOG=info
    depends_on:
      - db
      - redis
    restart: unless-stopped

  db:
    image: postgres:15
    environment:
      - POSTGRES_DB=myapp
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - app
    restart: unless-stopped

volumes:
  postgres_data:
```

### 配置管理

```rust
// src/config/settings.rs
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "pretty"
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        
        let s = Config::builder()
            // 默认配置
            .add_source(File::with_name("config/default"))
            // 环境特定配置
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // 本地配置（不提交到版本控制）
            .add_source(File::with_name("config/local").required(false))
            // 环境变量覆盖
            .add_source(Environment::with_prefix("APP").separator("_"))
            .build()?;
        
        s.try_deserialize()
    }
}
```

## 🎯 总结

这份最佳实践指南涵盖了使用 Rust 和 Axum 开发 Web 应用程序的关键方面：

1. **项目结构**：清晰的模块化组织
2. **代码组织**：分层架构和职责分离
3. **错误处理**：统一的错误类型和处理策略
4. **性能优化**：数据库、缓存和响应优化
5. **安全实践**：认证、授权和输入验证
6. **测试策略**：全面的单元测试和集成测试
7. **日志监控**：结构化日志和健康检查
8. **部署运维**：容器化和配置管理

遵循这些最佳实践将帮助你构建高质量、可维护、安全的 Rust Web 应用程序。记住，最佳实践会随着技术发展而演进，保持学习和更新是很重要的！