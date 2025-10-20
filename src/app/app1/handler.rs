//! App1 处理器

use axum::{
    extract::{Path, State},
    response::Json,
};
use uuid::Uuid;

use super::{model::User, service::UserService};
use crate::core::response::ApiResponse;

#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService,
}

pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<User>>, crate::core::error::CoreError> {
    let user = state.user_service.get_user(id)?;
    Ok(Json(ApiResponse::success(user)))
}

pub async fn create_user(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<User>>, crate::core::error::CoreError> {
    let user = state.user_service.create_user(
        "张三".to_string(),
        "zhangsan@example.com".to_string(),
        25,
    )?;
    Ok(Json(ApiResponse::success(user)))
}
