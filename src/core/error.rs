//! 核心错误处理模块

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

/// 核心错误类型
#[derive(Debug)]
pub enum CoreError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    InternalServerError(String),
}

/// 错误响应结构
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub code: u16,
    pub timestamp: String,
}

impl IntoResponse for CoreError {
    fn into_response(self) -> Response {
        let (status, error_message, user_message) = match self {
            CoreError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            CoreError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "认证失败，请提供有效的认证信息".to_string(),
            ),
            CoreError::Forbidden => (
                StatusCode::FORBIDDEN,
                "FORBIDDEN",
                "权限不足，无法访问此资源".to_string(),
            ),
            CoreError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            CoreError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
                msg,
            ),
        };

        let error_response = ErrorResponse {
            error: error_message.to_string(),
            message: user_message,
            code: status.as_u16(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        (status, axum::Json(error_response)).into_response()
    }
}
