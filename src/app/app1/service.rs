//! App1 业务服务

use super::model::User;
use crate::core::error::CoreError;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserService;

impl UserService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_user(&self, id: Uuid) -> Result<User, CoreError> {
        // 模拟从数据库获取用户
        Ok(User {
            id,
            name: "张三".to_string(),
            email: "zhangsan@example.com".to_string(),
            age: 25,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn create_user(&self, name: String, email: String, age: u32) -> Result<User, CoreError> {
        // 模拟创建用户
        let user = User {
            id: Uuid::new_v4(),
            name,
            email,
            age,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        Ok(user)
    }
}