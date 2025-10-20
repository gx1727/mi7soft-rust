//! App2 业务服务

use super::model::Product;
use crate::core::error::CoreError;
use uuid::Uuid;

#[derive(Clone)]
pub struct ProductService;

impl ProductService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_product(&self, id: Uuid) -> Result<Product, CoreError> {
        // 模拟从数据库获取产品
        Ok(Product {
            id,
            name: "示例产品".to_string(),
            description: "这是一个示例产品".to_string(),
            price: 99.99,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn create_product(
        &self,
        name: String,
        description: String,
        price: f64,
    ) -> Result<Product, CoreError> {
        // 模拟创建产品
        let product = Product {
            id: Uuid::new_v4(),
            name,
            description,
            price,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        Ok(product)
    }
}
