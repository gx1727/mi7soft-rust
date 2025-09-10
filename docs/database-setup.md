# 数据库设置指南

本文档介绍如何为 Rust + Axum 项目设置和配置数据库，主要使用 PostgreSQL 和 SQLx。

## 📋 目录

1. [PostgreSQL 安装](#postgresql-安装)
2. [数据库配置](#数据库配置)
3. [SQLx 使用指南](#sqlx-使用指南)
4. [迁移管理](#迁移管理)
5. [连接池配置](#连接池配置)
6. [最佳实践](#最佳实践)
7. [故障排除](#故障排除)

## PostgreSQL 安装

### Windows

1. **下载 PostgreSQL**
   - 访问 [PostgreSQL 官网](https://www.postgresql.org/download/windows/)
   - 下载适合你系统的安装程序

2. **安装步骤**
   ```bash
   # 运行安装程序，设置以下信息：
   # - 端口：5432（默认）
   # - 超级用户密码：记住这个密码
   # - 区域设置：选择合适的区域
   ```

3. **验证安装**
   ```bash
   # 打开命令提示符或 PowerShell
   psql --version
   ```

### macOS

```bash
# 使用 Homebrew
brew install postgresql
brew services start postgresql

# 或使用 Postgres.app
# 下载：https://postgresapp.com/
```

### Linux (Ubuntu/Debian)

```bash
# 更新包列表
sudo apt update

# 安装 PostgreSQL
sudo apt install postgresql postgresql-contrib

# 启动服务
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

### Docker 方式（推荐用于开发）

```bash
# 拉取 PostgreSQL 镜像
docker pull postgres:15

# 运行 PostgreSQL 容器
docker run --name postgres-dev \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=axum_example \
  -p 5432:5432 \
  -d postgres:15

# 或使用 Docker Compose
```

**docker-compose.yml 示例：**

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: axum_example
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./init.sql:/docker-entrypoint-initdb.d/init.sql
    restart: unless-stopped

volumes:
  postgres_data:
```

## 数据库配置

### 1. 创建数据库和用户

```sql
-- 连接到 PostgreSQL
psql -U postgres -h localhost

-- 创建数据库
CREATE DATABASE axum_example;

-- 创建用户（可选）
CREATE USER axum_user WITH PASSWORD 'secure_password';

-- 授予权限
GRANT ALL PRIVILEGES ON DATABASE axum_example TO axum_user;

-- 切换到新数据库
\c axum_example;

-- 授予 schema 权限
GRANT ALL ON SCHEMA public TO axum_user;
```

### 2. 环境变量配置

创建 `.env` 文件：

```bash
# .env
DATABASE_URL=postgres://postgres:password@localhost:5432/axum_example
TEST_DATABASE_URL=postgres://postgres:password@localhost:5432/axum_test
RUST_LOG=debug
```

**注意：** 将 `.env` 添加到 `.gitignore` 文件中，不要提交到版本控制。

### 3. 配置结构体

```rust
// src/config/database.rs
use serde::Deserialize;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://postgres:password@localhost:5432/axum_example".to_string(),
            max_connections: 20,
            min_connections: 5,
            acquire_timeout_seconds: 8,
            idle_timeout_seconds: 8,
            max_lifetime_seconds: 8,
        }
    }
}

impl DatabaseConfig {
    pub async fn create_pool(&self) -> Result<PgPool, sqlx::Error> {
        PgPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(Duration::from_secs(self.acquire_timeout_seconds))
            .idle_timeout(Duration::from_secs(self.idle_timeout_seconds))
            .max_lifetime(Duration::from_secs(self.max_lifetime_seconds))
            .connect(&self.url)
            .await
    }
}
```

## SQLx 使用指南

### 1. 基本查询

```rust
use sqlx::{PgPool, Row};
use uuid::Uuid;

// 简单查询
pub async fn get_user_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users")
        .fetch_one(pool)
        .await?;
    
    Ok(row.get("count"))
}

// 参数化查询
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, name, email, created_at FROM users WHERE email = $1"
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

// 插入数据
pub async fn create_user(pool: &PgPool, name: &str, email: &str) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *"
    )
    .bind(name)
    .bind(email)
    .fetch_one(pool)
    .await?;
    
    Ok(user)
}

// 更新数据
pub async fn update_user_name(pool: &PgPool, id: Uuid, name: &str) -> Result<User, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET name = $1, updated_at = NOW() WHERE id = $2 RETURNING *"
    )
    .bind(name)
    .bind(id)
    .fetch_one(pool)
    .await?;
    
    Ok(user)
}

// 删除数据
pub async fn delete_user(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}
```

### 2. 事务处理

```rust
use sqlx::{PgPool, Postgres, Transaction};

// 简单事务
pub async fn transfer_points(
    pool: &PgPool,
    from_user: Uuid,
    to_user: Uuid,
    points: i32,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    
    // 检查发送者余额
    let sender_balance: (i32,) = sqlx::query_as(
        "SELECT points FROM users WHERE id = $1"
    )
    .bind(from_user)
    .fetch_one(&mut *tx)
    .await?;
    
    if sender_balance.0 < points {
        tx.rollback().await?;
        return Err(sqlx::Error::RowNotFound);
    }
    
    // 扣除发送者积分
    sqlx::query("UPDATE users SET points = points - $1 WHERE id = $2")
        .bind(points)
        .bind(from_user)
        .execute(&mut *tx)
        .await?;
    
    // 增加接收者积分
    sqlx::query("UPDATE users SET points = points + $1 WHERE id = $2")
        .bind(points)
        .bind(to_user)
        .execute(&mut *tx)
        .await?;
    
    tx.commit().await?;
    Ok(())
}

// 复杂事务
pub async fn create_user_with_profile(
    pool: &PgPool,
    user_data: CreateUserRequest,
    profile_data: CreateProfileRequest,
) -> Result<(User, Profile), sqlx::Error> {
    let mut tx = pool.begin().await?;
    
    // 创建用户
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *"
    )
    .bind(&user_data.name)
    .bind(&user_data.email)
    .fetch_one(&mut *tx)
    .await?;
    
    // 创建用户资料
    let profile = sqlx::query_as::<_, Profile>(
        "INSERT INTO profiles (user_id, bio, avatar_url) VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(user.id)
    .bind(&profile_data.bio)
    .bind(&profile_data.avatar_url)
    .fetch_one(&mut *tx)
    .await?;
    
    tx.commit().await?;
    Ok((user, profile))
}
```

### 3. 批量操作

```rust
// 批量插入
pub async fn create_users_batch(
    pool: &PgPool,
    users: Vec<CreateUserRequest>,
) -> Result<Vec<User>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let mut created_users = Vec::new();
    
    for user_req in users {
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *"
        )
        .bind(&user_req.name)
        .bind(&user_req.email)
        .fetch_one(&mut *tx)
        .await?;
        
        created_users.push(user);
    }
    
    tx.commit().await?;
    Ok(created_users)
}

// 使用 UNNEST 进行批量插入（更高效）
pub async fn bulk_insert_users(
    pool: &PgPool,
    users: Vec<(String, String)>, // (name, email)
) -> Result<Vec<User>, sqlx::Error> {
    let names: Vec<String> = users.iter().map(|(name, _)| name.clone()).collect();
    let emails: Vec<String> = users.iter().map(|(_, email)| email.clone()).collect();
    
    let users = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (name, email)
        SELECT * FROM UNNEST($1::text[], $2::text[])
        RETURNING *
        "#
    )
    .bind(&names)
    .bind(&emails)
    .fetch_all(pool)
    .await?;
    
    Ok(users)
}
```

## 迁移管理

### 1. 安装 SQLx CLI

```bash
# 安装 SQLx CLI
cargo install sqlx-cli --no-default-features --features postgres

# 验证安装
sqlx --version
```

### 2. 初始化迁移

```bash
# 创建迁移目录
sqlx migrate add initial_schema

# 这会创建 migrations/TIMESTAMP_initial_schema.sql 文件
```

### 3. 编写迁移文件

**migrations/001_initial_schema.sql:**

```sql
-- 创建用户表
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255),
    age INTEGER CHECK (age > 0 AND age <= 150),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);

-- 创建更新时间触发器
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- 创建用户资料表
CREATE TABLE profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    bio TEXT,
    avatar_url VARCHAR(500),
    website VARCHAR(255),
    location VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_profiles_user_id ON profiles(user_id);

CREATE TRIGGER update_profiles_updated_at
    BEFORE UPDATE ON profiles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
```

### 4. 运行迁移

```bash
# 运行所有待执行的迁移
sqlx migrate run

# 检查迁移状态
sqlx migrate info

# 回滚最后一个迁移
sqlx migrate revert
```

### 5. 在代码中运行迁移

```rust
// src/database/migrations.rs
use sqlx::PgPool;

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

// 在 main.rs 中使用
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = create_database_pool().await?;
    
    // 运行迁移
    run_migrations(&pool).await?;
    
    // 启动应用...
    Ok(())
}
```

## 连接池配置

### 1. 生产环境配置

```rust
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub async fn create_production_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(50)                     // 最大连接数
        .min_connections(10)                     // 最小连接数
        .acquire_timeout(Duration::from_secs(30)) // 获取连接超时
        .idle_timeout(Duration::from_secs(600))   // 空闲连接超时（10分钟）
        .max_lifetime(Duration::from_secs(1800))  // 连接最大生命周期（30分钟）
        .test_before_acquire(true)               // 获取前测试连接
        .connect(database_url)
        .await
}
```

### 2. 开发环境配置

```rust
pub async fn create_development_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .connect(database_url)
        .await
}
```

### 3. 测试环境配置

```rust
pub async fn create_test_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await
}
```

## 最佳实践

### 1. 查询优化

```rust
// ✅ 好的做法：使用索引
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_status_created_at ON users(status, created_at);

// ✅ 好的做法：限制查询结果
pub async fn get_recent_users(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1"
    )
    .bind(limit.min(100)) // 限制最大值
    .fetch_all(pool)
    .await
}

// ✅ 好的做法：使用分页
pub async fn get_users_paginated(
    pool: &PgPool,
    page: u32,
    per_page: u32,
) -> Result<(Vec<User>, i64), sqlx::Error> {
    let offset = (page - 1) * per_page;
    let per_page = per_page.min(100); // 限制每页最大数量
    
    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await?;
    
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    
    Ok((users, total.0))
}
```

### 2. 错误处理

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(#[from] sqlx::Error),
    
    #[error("Record not found")]
    NotFound,
    
    #[error("Duplicate entry: {0}")]
    DuplicateEntry(String),
    
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => DatabaseError::NotFound,
            sqlx::Error::Database(db_err) => {
                if db_err.constraint().is_some() {
                    DatabaseError::ConstraintViolation(db_err.to_string())
                } else {
                    DatabaseError::ConnectionFailed(err)
                }
            }
            _ => DatabaseError::ConnectionFailed(err),
        }
    }
}
```

### 3. 测试策略

```rust
// tests/database_tests.rs
use sqlx::PgPool;
use testcontainers::clients::Cli;
use testcontainers::images::postgres::Postgres;

#[tokio::test]
async fn test_user_operations() {
    let docker = Cli::default();
    let postgres_image = Postgres::default();
    let node = docker.run(postgres_image);
    
    let connection_string = format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        node.get_host_port_ipv4(5432)
    );
    
    let pool = PgPool::connect(&connection_string).await.unwrap();
    
    // 运行迁移
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    
    // 测试用户创建
    let user = create_user(&pool, "Test User", "test@example.com")
        .await
        .unwrap();
    
    assert_eq!(user.name, "Test User");
    assert_eq!(user.email, "test@example.com");
    
    // 测试用户查询
    let found_user = get_user_by_email(&pool, "test@example.com")
        .await
        .unwrap()
        .unwrap();
    
    assert_eq!(found_user.id, user.id);
}
```

### 4. 性能监控

```rust
use tracing::{info, warn};
use std::time::Instant;

// 查询性能监控
pub async fn monitored_query<T>(
    pool: &PgPool,
    query: &str,
    operation: &str,
) -> Result<T, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let start = Instant::now();
    
    let result = sqlx::query_as::<_, T>(query)
        .fetch_one(pool)
        .await;
    
    let duration = start.elapsed();
    
    if duration.as_millis() > 1000 {
        warn!(
            "Slow query detected: {} took {}ms",
            operation,
            duration.as_millis()
        );
    } else {
        info!(
            "Query completed: {} took {}ms",
            operation,
            duration.as_millis()
        );
    }
    
    result
}
```

## 故障排除

### 常见问题

1. **连接被拒绝**
   ```
   Error: Connection refused (os error 61)
   ```
   - 检查 PostgreSQL 是否正在运行
   - 验证端口号和主机地址
   - 检查防火墙设置

2. **认证失败**
   ```
   Error: password authentication failed
   ```
   - 验证用户名和密码
   - 检查 `pg_hba.conf` 配置
   - 确认用户权限

3. **数据库不存在**
   ```
   Error: database "axum_example" does not exist
   ```
   - 创建数据库：`CREATE DATABASE axum_example;`
   - 检查数据库名称拼写

4. **连接池耗尽**
   ```
   Error: timed out while waiting for an open connection
   ```
   - 增加最大连接数
   - 检查连接泄漏
   - 优化查询性能

### 调试技巧

```rust
// 启用 SQLx 查询日志
std::env::set_var("SQLX_LOGGING", "true");

// 在 Cargo.toml 中添加
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid", "logging"] }

// 使用 tracing 记录数据库操作
use tracing::{instrument, info, error};

#[instrument(skip(pool))]
pub async fn get_user_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<User>, sqlx::Error> {
    info!("Fetching user with id: {}", id);
    
    let result = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await;
    
    match &result {
        Ok(Some(_)) => info!("User found"),
        Ok(None) => info!("User not found"),
        Err(e) => error!("Database error: {}", e),
    }
    
    result
}
```

## 🎯 总结

这份数据库设置指南涵盖了：

1. **PostgreSQL 安装**：多平台安装方法
2. **数据库配置**：用户、权限和环境变量
3. **SQLx 使用**：查询、事务和批量操作
4. **迁移管理**：版本控制和自动化
5. **连接池配置**：不同环境的优化设置
6. **最佳实践**：性能、错误处理和测试
7. **故障排除**：常见问题和解决方案

遵循这些指南将帮助你构建稳定、高性能的数据库层！