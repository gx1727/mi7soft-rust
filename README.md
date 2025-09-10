# Rust + Axum 学习项目

这是一个从零开始学习 Rust 和 Axum Web 框架的完整项目。通过一系列渐进式的示例，帮助你掌握 Rust 语言和现代 Web 开发技能。

## 📚 项目结构

```
rust-axum-learning/
├── Cargo.toml              # 项目配置和依赖
├── README.md               # 项目说明文档
├── docs/                   # 学习文档
│   ├── rust-basics.md      # Rust 基础语法
│   ├── axum-guide.md       # Axum 框架指南
│   └── best-practices.md   # 最佳实践
└── src/
    ├── main.rs             # 主程序入口
    └── bin/                # 示例程序
        ├── hello_world.rs      # Rust 基础示例
        ├── basic_server.rs     # 基础 Web 服务器
        ├── rest_api.rs         # REST API 示例
        ├── middleware_example.rs # 中间件和错误处理
        └── database_example.rs  # 数据库集成示例
```

## 🚀 快速开始

### 前置要求

- [Rust](https://rustup.rs/) (最新稳定版)
- 基本的命令行操作知识

### 安装和运行

1. **克隆项目**
   ```bash
   git clone <your-repo-url>
   cd rust-axum-learning
   ```

2. **安装依赖**
   ```bash
   cargo build
   ```

3. **运行示例程序**
   ```bash
   # Rust 基础示例
   cargo run --bin hello_world
   
   # 基础 Web 服务器 (http://127.0.0.1:3000)
   cargo run --bin basic_server
   
   # REST API 服务器 (http://127.0.0.1:3001)
   cargo run --bin rest_api
   
   # 中间件示例服务器 (http://127.0.0.1:3002)
   cargo run --bin middleware_example
   ```

## 📖 学习路径

### 第一步：Rust 基础 (`hello_world.rs`)

学习 Rust 的基本语法和概念：
- 变量和数据类型
- 函数定义和调用
- 控制流（if/else, loop, match）
- 所有权和借用
- 结构体和枚举

**运行命令：**
```bash
cargo run --bin hello_world
```

### 第二步：基础 Web 服务器 (`basic_server.rs`)

创建你的第一个 Axum Web 服务器：
- HTTP 路由配置
- 处理器函数（Handler）
- JSON 响应
- 查询参数和路径参数
- 静态文件服务

**运行命令：**
```bash
cargo run --bin basic_server
```

**测试端点：**
- `GET /` - 主页
- `GET /hello/:name` - 个性化问候
- `GET /json` - JSON 响应示例
- `GET /user/:id` - 用户信息
- `POST /echo` - 回显请求体
- `GET /health` - 健康检查

### 第三步：REST API (`rest_api.rs`)

构建完整的 RESTful API：
- CRUD 操作（创建、读取、更新、删除）
- 数据验证
- 错误处理
- 状态管理
- CORS 支持

**运行命令：**
```bash
cargo run --bin rest_api
```

**API 端点：**
- `GET /users` - 获取所有用户
- `GET /users/:id` - 获取特定用户
- `POST /users` - 创建新用户
- `PUT /users/:id` - 更新用户
- `DELETE /users/:id` - 删除用户

**示例请求：**
```bash
# 获取所有用户
curl http://127.0.0.1:3001/users

# 创建新用户
curl -X POST http://127.0.0.1:3001/users \
  -H "Content-Type: application/json" \
  -d '{"name":"张三","email":"zhangsan@example.com","age":25}'
```

### 第四步：中间件和错误处理 (`middleware_example.rs`)

学习高级 Web 开发概念：
- 自定义中间件
- 请求日志记录
- 身份验证
- 限流
- 超时处理
- 错误处理和响应

**运行命令：**
```bash
cargo run --bin middleware_example
```

**特殊端点：**
- `GET /protected` - 需要 Authorization header
- `GET /slow` - 慢响应测试（3秒延迟）
- `GET /error` - 错误处理示例
- `GET /stats` - 服务器统计信息

**测试认证：**
```bash
curl -H "Authorization: Bearer your-token" http://127.0.0.1:3002/protected
```

## 🛠️ 技术栈

- **[Rust](https://www.rust-lang.org/)** - 系统编程语言
- **[Axum](https://github.com/tokio-rs/axum)** - 现代异步 Web 框架
- **[Tokio](https://tokio.rs/)** - 异步运行时
- **[Serde](https://serde.rs/)** - 序列化/反序列化
- **[Tower](https://github.com/tower-rs/tower)** - 中间件和服务抽象
- **[Tracing](https://tracing.rs/)** - 结构化日志记录

## 📝 核心概念

### Axum 路由

``` rust
let app = Router::new()
    .route("/", get(handler))           // GET 请求
    .route("/users", post(create_user)) // POST 请求
    .route("/users/:id", get(get_user)) // 路径参数
    .with_state(app_state);              // 共享状态
```

### 处理器函数

``` rust
// 简单处理器
async fn hello() -> &'static str {
    "Hello, World!"
}

// 带参数的处理器
async fn get_user(Path(id): Path<u32>) -> Json<User> {
    // 处理逻辑
}

// 带状态的处理器
async fn handler(State(state): State<AppState>) -> Response {
    // 使用共享状态
}
```

### 中间件

```rust
// 自定义中间件
async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 认证逻辑
    let response = next.run(request).await;
    Ok(response)
}

// 应用中间件
let app = Router::new()
    .route("/protected", get(handler))
    .layer(middleware::from_fn(auth_middleware));
```

## 🎯 学习目标

完成这个项目后，你将掌握：

- ✅ Rust 语言基础语法和概念
- ✅ Axum Web 框架的核心功能
- ✅ 异步编程模式
- ✅ RESTful API 设计和实现
- ✅ 中间件开发
- ✅ 错误处理最佳实践
- ✅ Web 安全基础
- ✅ 现代 Web 开发工具链

## 🔗 有用的资源

- [Rust 官方教程](https://doc.rust-lang.org/book/)
- [Axum 官方文档](https://docs.rs/axum/latest/axum/)
- [Tokio 教程](https://tokio.rs/tokio/tutorial)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Axum 示例集合](https://github.com/tokio-rs/axum/tree/main/examples)

## 🤝 贡献

欢迎提交 Issue 和 Pull Request 来改进这个学习项目！

## 📄 许可证

MIT License

---

**开始你的 Rust + Axum 学习之旅吧！** 