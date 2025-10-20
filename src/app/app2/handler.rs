//! App2 处理器

use axum::{
    extract::{Path, State},
    response::Json,
};
use uuid::Uuid;

use super::{model::Product, service::ProductService};
use crate::core::response::ApiResponse;

#[derive(Clone)]
pub struct AppState {
    pub product_service: ProductService,
}

pub async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Product>>, crate::core::error::CoreError> {
    let product = state.product_service.get_product(id)?;
    Ok(Json(ApiResponse::success(product)))
}

pub async fn create_product(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Product>>, crate::core::error::CoreError> {
    let product = state.product_service.create_product(
        "示例产品".to_string(),
        "这是一个示例产品".to_string(),
        99.99,
    )?;
    Ok(Json(ApiResponse::success(product)))
}
