//! 数据库集成示例
//! 
//! 本示例展示如何在 Axum 应用中集成 SQLx 进行数据库操作，
//! 包括连接池管理、CRUD 操作、事务处理等。

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    Row,
};
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, error};
use uuid::Uuid;
use validator::Validate;

// 应用状态
#[derive(Clone)]
struct AppState {
    db: PgPool,
}

// 用户模型
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct User {
    id: Uuid,
    name: String,
    email: String,
    age: Option<i32>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

// 创建用户请求
#[derive(Debug, Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(length(min = 2, max = 50, message = "Name must be between 2 and 50 characters"))]
    name: String,
    
    #[validate(email(message = "Invalid email format"))]
    email: String,
    
    #[validate(range(min = 1, max = 150, message = "Age must be between 1 and 150"))]
    age: Option<i32>,
}

// 更新用户请求
#[derive(Debug, Deserialize, Validate)]
struct UpdateUserRequest {
    #[validate(length(min = 2, max = 50, message = "Name must be between 2 and 50 characters"))]
    name: Option<String>,
    
    #[validate(email(message = "Invalid email format"))]
    email: Option<String>,
    
    #[validate(range(min = 1, max = 150, message = "Age must be between 1 and 150"))]
    age: Option<i32>,
}

// 查询参数
#[derive(Debug, Deserialize)]
struct ListUsersQuery {
    page: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
}

// 分页响应
#[derive(Debug, Serialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
struct PaginationInfo {
    page: u32,
    limit: u32,
    total: i64,
    total_pages: u32,
}

// 自定义错误类型
#[derive(Debug)]
enum AppError {
    Database(sqlx::Error),
    NotFound,
    ValidationError(String),
    InternalError(String),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::Database(e) => {
                error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };
        
        (status, Json(serde_json::json!({
            "error": message
        }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        let messages: Vec<String> = err
            .field_errors()
            .into_iter()
            .flat_map(|(_, errors)| {
                errors.iter().map(|error| {
                    error.message
                        .as_ref()
                        .map(|msg| msg.to_string())
                        .unwrap_or_else(|| "Validation error".to_string())
                })
            })
            .collect();
        
        AppError::ValidationError(messages.join(", "))
    }
}

// 数据库初始化
async fn init_database() -> Result<PgPool, sqlx::Error> {
    // 在实际应用中，这应该从环境变量或配置文件读取
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/axum_example".to_string());
    
    info!("Connecting to database: {}", database_url.replace(":password@", ":***@"));
    
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .connect(&database_url)
        .await?;
    
    // 创建表（在实际应用中应该使用迁移）
    create_tables(&pool).await?;
    
    // 插入示例数据
    seed_data(&pool).await?;
    
    Ok(pool)
}

// 创建数据库表
async fn create_tables(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Creating database tables...");
    
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(50) NOT NULL,
            email VARCHAR(100) UNIQUE NOT NULL,
            age INTEGER CHECK (age > 0 AND age <= 150),
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;
    
    // 创建更新时间触发器函数
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION update_updated_at_column()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = NOW();
            RETURN NEW;
        END;
        $$ language 'plpgsql'
        "#,
    )
    .execute(pool)
    .await?;
    
    // 删除已存在的触发器
    sqlx::query("DROP TRIGGER IF EXISTS update_users_updated_at ON users")
        .execute(pool)
        .await?;
    
    // 创建新的触发器
    sqlx::query(
        r#"
        CREATE TRIGGER update_users_updated_at
            BEFORE UPDATE ON users
            FOR EACH ROW
            EXECUTE FUNCTION update_updated_at_column()
        "#,
    )
    .execute(pool)
    .await?;
    
    info!("Database tables created successfully");
    Ok(())
}

// 插入示例数据
async fn seed_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Seeding database with example data...");
    
    // 检查是否已有数据
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    
    if count.0 > 0 {
        info!("Database already contains data, skipping seed");
        return Ok(());
    }
    
    // 插入示例用户
    let users = vec![
        ("Alice Johnson", "alice@example.com", Some(28)),
        ("Bob Smith", "bob@example.com", Some(35)),
        ("Charlie Brown", "charlie@example.com", Some(22)),
        ("Diana Prince", "diana@example.com", None),
        ("Eve Wilson", "eve@example.com", Some(31)),
    ];
    
    for (name, email, age) in users {
        sqlx::query(
            "INSERT INTO users (name, email, age) VALUES ($1, $2, $3)"
        )
        .bind(name)
        .bind(email)
        .bind(age)
        .execute(pool)
        .await?;
    }
    
    info!("Database seeded successfully");
    Ok(())
}

// 处理器函数

// 获取用户列表（支持分页和搜索）
async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<PaginatedResponse<User>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(10).min(100).max(1);
    let offset = (page - 1) * limit;
    
    let (users, total) = if let Some(search) = &query.search {
        // 搜索用户
        let search_pattern = format!("%{}%", search);
        
        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE name ILIKE $1 OR email ILIKE $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(&search_pattern)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&state.db)
        .await?;
        
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE name ILIKE $1 OR email ILIKE $1"
        )
        .bind(&search_pattern)
        .fetch_one(&state.db)
        .await?;
        
        (users, total.0)
    } else {
        // 获取所有用户
        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&state.db)
        .await?;
        
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&state.db)
            .await?;
        
        (users, total.0)
    };
    
    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;
    
    Ok(Json(PaginatedResponse {
        data: users,
        pagination: PaginationInfo {
            page,
            limit,
            total,
            total_pages,
        },
    }))
}

// 根据 ID 获取用户
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    
    Ok(Json(user))
}

// 创建用户
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), AppError> {
    // 验证输入
    payload.validate()?;
    
    // 检查邮箱是否已存在
    let existing_user = sqlx::query("SELECT id FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db)
        .await?;
    
    if existing_user.is_some() {
        return Err(AppError::ValidationError("Email already exists".to_string()));
    }
    
    // 创建用户
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(&payload.name)
    .bind(&payload.email)
    .bind(payload.age)
    .fetch_one(&state.db)
    .await?;
    
    info!("Created user: {} ({})", user.name, user.id);
    
    Ok((StatusCode::CREATED, Json(user)))
}

// 更新用户
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<User>, AppError> {
    // 验证输入
    payload.validate()?;
    
    // 检查用户是否存在
    let _existing_user = sqlx::query("SELECT id FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    
    // 如果更新邮箱，检查是否与其他用户冲突
    if let Some(ref email) = payload.email {
        let email_conflict = sqlx::query("SELECT id FROM users WHERE email = $1 AND id != $2")
            .bind(email)
            .bind(id)
            .fetch_optional(&state.db)
            .await?;
        
        if email_conflict.is_some() {
            return Err(AppError::ValidationError("Email already exists".to_string()));
        }
    }
    
    // 构建动态更新查询
    let mut query = "UPDATE users SET ".to_string();
    let mut params = Vec::new();
    let mut param_count = 1;
    
    if let Some(name) = &payload.name {
        query.push_str(&format!("name = ${}, ", param_count));
        params.push(name as &(dyn sqlx::Encode<sqlx::Postgres> + Sync));
        param_count += 1;
    }
    
    if let Some(email) = &payload.email {
        query.push_str(&format!("email = ${}, ", param_count));
        params.push(email as &(dyn sqlx::Encode<sqlx::Postgres> + Sync));
        param_count += 1;
    }
    
    if let Some(age) = &payload.age {
        query.push_str(&format!("age = ${}, ", param_count));
        params.push(age as &(dyn sqlx::Encode<sqlx::Postgres> + Sync));
        param_count += 1;
    }
    
    if params.is_empty() {
        return Err(AppError::ValidationError("No fields to update".to_string()));
    }
    
    // 移除最后的逗号和空格
    query.truncate(query.len() - 2);
    query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));
    
    // 使用事务进行更新
    let mut tx = state.db.begin().await?;
    
    let user = if let (Some(name), Some(email), Some(age)) = (&payload.name, &payload.email, &payload.age) {
        sqlx::query_as::<_, User>(
            "UPDATE users SET name = $1, email = $2, age = $3 WHERE id = $4 RETURNING *"
        )
        .bind(name)
        .bind(email)
        .bind(age)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?
    } else if let (Some(name), Some(email)) = (&payload.name, &payload.email) {
        sqlx::query_as::<_, User>(
            "UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING *"
        )
        .bind(name)
        .bind(email)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?
    } else if let Some(name) = &payload.name {
        sqlx::query_as::<_, User>(
            "UPDATE users SET name = $1 WHERE id = $2 RETURNING *"
        )
        .bind(name)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?
    } else if let Some(email) = &payload.email {
        sqlx::query_as::<_, User>(
            "UPDATE users SET email = $1 WHERE id = $2 RETURNING *"
        )
        .bind(email)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?
    } else if let Some(age) = &payload.age {
        sqlx::query_as::<_, User>(
            "UPDATE users SET age = $1 WHERE id = $2 RETURNING *"
        )
        .bind(age)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?
    } else {
        return Err(AppError::ValidationError("No fields to update".to_string()));
    };
    
    tx.commit().await?;
    
    info!("Updated user: {} ({})", user.name, user.id);
    
    Ok(Json(user))
}

// 删除用户
async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    
    info!("Deleted user: {}", id);
    
    Ok(StatusCode::NO_CONTENT)
}

// 批量操作示例
async fn create_users_batch(
    State(state): State<AppState>,
    Json(users): Json<Vec<CreateUserRequest>>,
) -> Result<(StatusCode, Json<Vec<User>>), AppError> {
    if users.is_empty() {
        return Err(AppError::ValidationError("No users provided".to_string()));
    }
    
    if users.len() > 100 {
        return Err(AppError::ValidationError("Too many users (max 100)".to_string()));
    }
    
    // 验证所有用户
    for user in &users {
        user.validate()?;
    }
    
    // 使用事务批量创建
    let mut tx = state.db.begin().await?;
    let mut created_users = Vec::new();
    
    for user_req in users {
        // 检查邮箱是否已存在
        let existing = sqlx::query("SELECT id FROM users WHERE email = $1")
            .bind(&user_req.email)
            .fetch_optional(&mut *tx)
            .await?;
        
        if existing.is_some() {
            tx.rollback().await?;
            return Err(AppError::ValidationError(
                format!("Email {} already exists", user_req.email)
            ));
        }
        
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(&user_req.name)
        .bind(&user_req.email)
        .bind(user_req.age)
        .fetch_one(&mut *tx)
        .await?;
        
        created_users.push(user);
    }
    
    tx.commit().await?;
    
    info!("Created {} users in batch", created_users.len());
    
    Ok((StatusCode::CREATED, Json(created_users)))
}

// 数据库统计信息
async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    
    let avg_age: (Option<f64>,) = sqlx::query_as("SELECT AVG(age) FROM users WHERE age IS NOT NULL")
        .fetch_one(&state.db)
        .await?;
    
    let users_by_age_group = sqlx::query(
        r#"
        SELECT 
            CASE 
                WHEN age < 20 THEN 'Under 20'
                WHEN age BETWEEN 20 AND 29 THEN '20-29'
                WHEN age BETWEEN 30 AND 39 THEN '30-39'
                WHEN age BETWEEN 40 AND 49 THEN '40-49'
                WHEN age >= 50 THEN '50+'
                ELSE 'Unknown'
            END as age_group,
            COUNT(*) as count
        FROM users 
        GROUP BY age_group
        ORDER BY age_group
        "#
    )
    .fetch_all(&state.db)
    .await?;
    
    let mut age_groups = serde_json::Map::new();
    for row in users_by_age_group {
        let age_group: String = row.get("age_group");
        let count: i64 = row.get("count");
        age_groups.insert(age_group, serde_json::Value::Number(count.into()));
    }
    
    Ok(Json(serde_json::json!({
        "total_users": total_users.0,
        "average_age": avg_age.0,
        "users_by_age_group": age_groups
    })))
}

// 健康检查
async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 测试数据库连接
    sqlx::query("SELECT 1")
        .execute(&state.db)
        .await?;
    
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "database": "connected",
        "timestamp": chrono::Utc::now()
    })))
}

// 创建路由
fn create_routes() -> Router<AppState> {
    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        // 用户 CRUD 操作
        .route("/users", get(list_users))
        .route("/users", post(create_user))
        .route("/users/batch", post(create_users_batch))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        // 统计信息
        .route("/stats", get(get_stats))
        // 添加追踪中间件
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();
    
    info!("Starting database example server...");
    
    // 初始化数据库
    let db = init_database().await.map_err(|e| {
        error!("Failed to initialize database: {}", e);
        e
    })?;
    
    // 创建应用状态
    let state = AppState { db };
    
    // 创建应用
    let app = create_routes().with_state(state);
    
    // 启动服务器
    let listener = TcpListener::bind("127.0.0.1:3003").await?;
    let addr = listener.local_addr()?;
    
    info!("🚀 Database example server running on http://{}", addr);
    info!("📊 Available endpoints:");
    info!("   GET    /health           - Health check");
    info!("   GET    /users            - List users (supports ?page=1&limit=10&search=term)");
    info!("   POST   /users            - Create user");
    info!("   POST   /users/batch      - Create multiple users");
    info!("   GET    /users/:id        - Get user by ID");
    info!("   PUT    /users/:id        - Update user");
    info!("   DELETE /users/:id        - Delete user");
    info!("   GET    /stats            - Database statistics");
    info!("");
    info!("💡 Example requests:");
    info!("   curl http://127.0.0.1:3003/users");
    info!("   curl http://127.0.0.1:3003/users?search=alice");
    info!("   curl -X POST http://127.0.0.1:3003/users -H 'Content-Type: application/json' -d '{{\"name\":\"John\",\"email\":\"john@example.com\",\"age\":30}}'");
    info!("   curl http://127.0.0.1:3003/stats");
    info!("");
    info!("⚠️  Note: This example requires a PostgreSQL database.");
    info!("   Set DATABASE_URL environment variable or use default: postgres://postgres:password@localhost/axum_example");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// 测试模块
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use serde_json::json;
    
    async fn create_test_app() -> Router {
        // 在测试中使用内存数据库或测试数据库
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:password@localhost/axum_test".to_string());
        
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database");
        
        create_tables(&pool).await.expect("Failed to create tables");
        
        let state = AppState { db: pool };
        create_routes().with_state(state)
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/health").await;
        response.assert_status_ok();
        
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "healthy");
    }
    
    #[tokio::test]
    async fn test_create_and_get_user() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        // 创建用户
        let create_request = json!({
            "name": "Test User",
            "email": "test@example.com",
            "age": 25
        });
        
        let response = server
            .post("/users")
            .json(&create_request)
            .await;
        
        response.assert_status(StatusCode::CREATED);
        let created_user: User = response.json();
        
        assert_eq!(created_user.name, "Test User");
        assert_eq!(created_user.email, "test@example.com");
        assert_eq!(created_user.age, Some(25));
        
        // 获取用户
        let response = server
            .get(&format!("/users/{}", created_user.id))
            .await;
        
        response.assert_status_ok();
        let fetched_user: User = response.json();
        assert_eq!(fetched_user.id, created_user.id);
    }
    
    #[tokio::test]
    async fn test_list_users() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let response = server.get("/users").await;
        response.assert_status_ok();
        
        let body: PaginatedResponse<User> = response.json();
        assert!(body.data.len() <= 10); // 默认限制
    }
    
    #[tokio::test]
    async fn test_validation_error() {
        let app = create_test_app().await;
        let server = TestServer::new(app).unwrap();
        
        let invalid_request = json!({
            "name": "", // 太短
            "email": "invalid-email", // 无效邮箱
            "age": 200 // 超出范围
        });
        
        let response = server
            .post("/users")
            .json(&invalid_request)
            .await;
        
        response.assert_status(StatusCode::BAD_REQUEST);
    }
}