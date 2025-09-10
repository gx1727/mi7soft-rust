//! REST API 示例
//! 演示完整的 CRUD 操作 (Create, Read, Update, Delete)
//! 使用内存存储模拟数据库操作

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use uuid::Uuid;

// 应用状态类型
type AppState = Arc<Mutex<HashMap<Uuid, User>>>;

// 用户数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Uuid,
    name: String,
    email: String,
    age: u32,
    created_at: String,
    updated_at: String,
}

// 创建用户请求
#[derive(Debug, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
    age: u32,
}

// 更新用户请求
#[derive(Debug, Deserialize)]
struct UpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
    age: Option<u32>,
}

// 查询参数
#[derive(Debug, Deserialize)]
struct UserQuery {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    min_age: Option<u32>,
    #[serde(default)]
    max_age: Option<u32>,
}

// API 响应包装器
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    message: String,
    timestamp: String,
}

// 错误响应
#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
    timestamp: String,
}

impl<T> ApiResponse<T> {
    fn success(data: T, message: &str) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: message.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl ErrorResponse {
    fn new(error: &str) -> Self {
        Self {
            success: false,
            error: error.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("启动 REST API 服务器...");

    // 创建共享状态
    let state: AppState = Arc::new(Mutex::new(HashMap::new()));

    // 添加一些示例数据
    init_sample_data(&state).await;

    // 创建路由
    let app = Router::new()
        .route("/", get(api_info))
        .route("/users", get(get_users).post(create_user))
        .route("/users/:id", get(get_user).put(update_user).delete(delete_user))
        .route("/users/:id/profile", get(get_user_profile))
        .route("/health", get(health_check))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 绑定地址
    let listener = TcpListener::bind("127.0.0.1:3001")
        .await
        .expect("无法绑定到端口 3001");

    info!("🚀 REST API 服务器运行在 http://127.0.0.1:3001");
    info!("📖 API 端点:");
    info!("   GET    /              - API 信息");
    info!("   GET    /users         - 获取所有用户 (支持查询参数)");
    info!("   POST   /users         - 创建新用户");
    info!("   GET    /users/:id     - 获取特定用户");
    info!("   PUT    /users/:id     - 更新用户");
    info!("   DELETE /users/:id     - 删除用户");
    info!("   GET    /users/:id/profile - 获取用户详细资料");
    info!("   GET    /health        - 健康检查");

    // 启动服务器
    axum::serve(listener, app)
        .await
        .expect("服务器启动失败");
}

/// 初始化示例数据
async fn init_sample_data(state: &AppState) {
    let mut users = state.lock().unwrap();
    
    let sample_users = vec![
        CreateUserRequest {
            name: "张三".to_string(),
            email: "zhangsan@example.com".to_string(),
            age: 25,
        },
        CreateUserRequest {
            name: "李四".to_string(),
            email: "lisi@example.com".to_string(),
            age: 30,
        },
        CreateUserRequest {
            name: "王五".to_string(),
            email: "wangwu@example.com".to_string(),
            age: 28,
        },
    ];

    for user_req in sample_users {
        let user = User {
            id: Uuid::new_v4(),
            name: user_req.name,
            email: user_req.email,
            age: user_req.age,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        users.insert(user.id, user);
    }
    
    info!("✅ 已初始化 {} 个示例用户", users.len());
}

/// API 信息
async fn api_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "Rust Axum REST API",
        "version": "0.1.0",
        "description": "学习 Rust 和 Axum 的 REST API 示例",
        "endpoints": {
            "users": {
                "GET /users": "获取所有用户，支持查询参数: name, min_age, max_age",
                "POST /users": "创建新用户",
                "GET /users/:id": "获取特定用户",
                "PUT /users/:id": "更新用户信息",
                "DELETE /users/:id": "删除用户",
                "GET /users/:id/profile": "获取用户详细资料"
            }
        },
        "examples": {
            "create_user": {
                "method": "POST",
                "url": "/users",
                "body": {
                    "name": "新用户",
                    "email": "newuser@example.com",
                    "age": 25
                }
            },
            "update_user": {
                "method": "PUT",
                "url": "/users/:id",
                "body": {
                    "name": "更新的名字",
                    "age": 26
                }
            },
            "query_users": {
                "method": "GET",
                "url": "/users?name=张&min_age=20&max_age=30"
            }
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// 获取所有用户 (支持查询过滤)
async fn get_users(
    State(state): State<AppState>,
    Query(query): Query<UserQuery>,
) -> Json<ApiResponse<Vec<User>>> {
    let users = state.lock().unwrap();
    let mut filtered_users: Vec<User> = users.values().cloned().collect();

    // 应用过滤器
    if let Some(name) = &query.name {
        filtered_users.retain(|user| user.name.contains(name));
    }
    
    if let Some(min_age) = query.min_age {
        filtered_users.retain(|user| user.age >= min_age);
    }
    
    if let Some(max_age) = query.max_age {
        filtered_users.retain(|user| user.age <= max_age);
    }

    // 按创建时间排序
    filtered_users.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let message = if filtered_users.len() == users.len() {
        format!("获取到 {} 个用户", filtered_users.len())
    } else {
        format!("过滤后获取到 {} 个用户 (总共 {} 个)", filtered_users.len(), users.len())
    };

    Json(ApiResponse::success(filtered_users, &message))
}

/// 获取特定用户
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, Json<ErrorResponse>)> {
    let users = state.lock().unwrap();
    
    match users.get(&id) {
        Some(user) => Ok(Json(ApiResponse::success(
            user.clone(),
            "用户获取成功",
        ))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(&format!("用户 {} 不存在", id))),
        )),
    }
}

/// 创建新用户
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<ApiResponse<User>>), (StatusCode, Json<ErrorResponse>)> {
    // 验证输入
    if payload.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("用户名不能为空")),
        ));
    }
    
    if payload.email.trim().is_empty() || !payload.email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("请提供有效的邮箱地址")),
        ));
    }

    let mut users = state.lock().unwrap();
    
    // 检查邮箱是否已存在
    for user in users.values() {
        if user.email == payload.email {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new("邮箱地址已存在")),
            ));
        }
    }

    let user = User {
        id: Uuid::new_v4(),
        name: payload.name.trim().to_string(),
        email: payload.email.trim().to_lowercase(),
        age: payload.age,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    users.insert(user.id, user.clone());
    
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(user, "用户创建成功")),
    ))
}

/// 更新用户
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, Json<ErrorResponse>)> {
    // 验证输入
    if let Some(ref name) = payload.name {
        if name.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("用户名不能为空")),
            ));
        }
    }
    
    if let Some(ref email) = payload.email {
        if email.trim().is_empty() || !email.contains('@') {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("请提供有效的邮箱地址")),
            ));
        }
    }
    
    let mut users = state.lock().unwrap();
    
    // 检查用户是否存在
    if !users.contains_key(&id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(&format!("用户 {} 不存在", id))),
        ));
    }
    
    // 如果要更新邮箱，检查是否已被其他用户使用
    if let Some(ref email) = payload.email {
        let email_trimmed = email.trim().to_lowercase();
        let email_exists = users.iter().any(|(other_id, other_user)| {
            *other_id != id && other_user.email == email_trimmed
        });
        
        if email_exists {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new("邮箱地址已被其他用户使用")),
            ));
        }
    }
    
    // 现在安全地获取可变引用并更新
    let user = users.get_mut(&id).unwrap();
    
    if let Some(name) = payload.name {
        user.name = name.trim().to_string();
    }
    
    if let Some(email) = payload.email {
        user.email = email.trim().to_lowercase();
    }
    
    if let Some(age) = payload.age {
        user.age = age;
    }
    
    user.updated_at = chrono::Utc::now().to_rfc3339();
    
    Ok(Json(ApiResponse::success(user.clone(), "用户更新成功")))
}

/// 删除用户
async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ErrorResponse>)> {
    let mut users = state.lock().unwrap();
    
    match users.remove(&id) {
        Some(_) => Ok(Json(ApiResponse::success((), "用户删除成功"))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(&format!("用户 {} 不存在", id))),
        )),
    }
}

/// 获取用户详细资料
async fn get_user_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let users = state.lock().unwrap();
    
    match users.get(&id) {
        Some(user) => {
            let profile = serde_json::json!({
                "user": user,
                "profile": {
                    "account_age_days": calculate_account_age(&user.created_at),
                    "last_updated_days_ago": calculate_days_since(&user.updated_at),
                    "email_domain": user.email.split('@').nth(1).unwrap_or("unknown"),
                    "age_group": categorize_age(user.age),
                    "status": "active"
                },
                "statistics": {
                    "total_users": users.len(),
                    "user_rank_by_age": calculate_age_rank(&users, user.age)
                }
            });
            
            Ok(Json(profile))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(&format!("用户 {} 不存在", id))),
        )),
    }
}

/// 健康检查
async fn health_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    let users = state.lock().unwrap();
    
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "0.1.0",
        "database": {
            "status": "connected",
            "type": "in-memory",
            "users_count": users.len()
        },
        "uptime": "运行中"
    }))
}

// 辅助函数
fn calculate_account_age(created_at: &str) -> i64 {
    if let Ok(created) = chrono::DateTime::parse_from_rfc3339(created_at) {
        let now = chrono::Utc::now();
        (now - created.with_timezone(&chrono::Utc)).num_days()
    } else {
        0
    }
}

fn calculate_days_since(updated_at: &str) -> i64 {
    if let Ok(updated) = chrono::DateTime::parse_from_rfc3339(updated_at) {
        let now = chrono::Utc::now();
        (now - updated.with_timezone(&chrono::Utc)).num_days()
    } else {
        0
    }
}

fn categorize_age(age: u32) -> &'static str {
    match age {
        0..=17 => "未成年",
        18..=25 => "青年",
        26..=35 => "青壮年",
        36..=50 => "中年",
        _ => "老年",
    }
}

fn calculate_age_rank(users: &HashMap<Uuid, User>, user_age: u32) -> usize {
    users.values().filter(|u| u.age <= user_age).count()
}