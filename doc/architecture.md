# 模块化分层架构设计

## 架构概述

本项目采用模块化的分层架构设计，主要分为以下几层：

1. **应用层** (`src/app/`)：包含具体业务逻辑实现
2. **服务层** (`src/core/service.rs`)：提供业务服务抽象
3. **核心层** (`src/core/`)：提供框架核心功能
4. **基础设施层** (`src/infrastructure/`)：提供底层支持

## 目录结构

```
├── bin/                # 可执行文件目录
├── cache/              # 缓存文件目录
├── cmd/                # 命令脚本目录
├── config/             # 配置文件目录
├── doc/                # 项目文档目录
├── logs/               # 日志文件目录
├── public/             # 公共资源目录
├── src/                # 源代码目录
│   ├── app/            # 应用目录
│   │   ├── app1/       # 应用目录1
│   │   │   ├── mod.rs  # 模块声明
│   │   │   ├── handler.rs # 处理器
│   │   │   ├── service.rs # 服务
│   │   │   └── model.rs  # 数据模型
│   │   ├── app2/       # 应用目录2
│   │   │   ├── mod.rs  # 模块声明
│   │   │   ├── handler.rs # 处理器
│   │   │   ├── service.rs # 服务
│   │   │   └── model.rs  # 数据模型
│   │   └── mod.rs      # 应用模块声明
│   ├── core/           # 核心代码目录
│   │   ├── mod.rs      # 核心模块声明
│   │   ├── error.rs    # 错误处理
│   │   ├── middleware.rs # 中间件
│   │   ├── response.rs # 响应处理
│   │   └── service.rs  # 服务抽象
│   ├── infrastructure/ # 基础设施目录
│   │   ├── mod.rs      # 基础设施模块声明
│   │   ├── database.rs # 数据库支持
│   │   └── logger.rs   # 日志支持
│   ├── lib.rs          # 库入口
│   └── main.rs         # 主程序入口
└── third_party/        # 第三方库目录
```

## 各层职责

### 应用层 (Application Layer)

- 包含具体业务逻辑实现
- 每个应用模块包含自己的处理器、服务和数据模型
- 处理HTTP请求和响应

### 服务层 (Service Layer)

- 提供业务服务抽象
- 定义业务逻辑接口
- 协调不同领域对象之间的交互

### 核心层 (Core Layer)

- 提供框架核心功能
- 包括错误处理、中间件、响应格式化等通用功能
- 被其他层共享使用

### 基础设施层 (Infrastructure Layer)

- 提供底层技术支持
- 包括数据库连接、日志记录、外部服务集成等
- 实现核心层定义的接口

## 运行项目

```bash
# 构建项目
cargo build

# 运行主服务器
cargo run

# 运行原有的示例程序
cargo run --bin hello_world
cargo run --bin basic_server
cargo run --bin rest_api
cargo run --bin middleware_example
```