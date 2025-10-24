# Mi7Soft HTTP 服务器集成指南

## 概述

Mi7Soft 现在支持通过 HTTP 接口接收消息并将其发送到跨进程消息队列，由 worker 进程处理。这个集成提供了一个简单而强大的 Web API 来与消息队列系统交互。

## 架构

```
HTTP 客户端 → HTTP 服务器 (entry) → 消息队列 → Worker 进程
```

## API 接口

### 1. Hello 接口
- **URL**: `GET /hello`
- **描述**: 简单的健康检查接口
- **响应**: `"Hello, Mi7Soft Message Queue!"`

### 2. 状态查询接口
- **URL**: `GET /status`
- **描述**: 查询服务器和消息队列状态
- **响应示例**:
```json
{
  "server": "Mi7Soft HTTP Server",
  "status": "running",
  "queue": {
    "queue_name": "task_queue",
    "capacity": 100,
    "current_size": 0,
    "status": "connected"
  }
}
```

### 3. 发送消息接口
- **URL**: `POST /send`
- **Content-Type**: `application/json`
- **请求体**:
```json
{
  "message": "要发送的消息内容",
  "data": {
    "type": "消息类型",
    "priority": 1
  }
}
```
- **响应示例**:
```json
{
  "success": true,
  "message": "消息已成功发送到队列",
  "task_id": 12345
}
```

### 4. 通用路径处理
- **URL**: `GET /{任意路径}`
- **描述**: 处理任意 GET 请求并发送到消息队列
- **响应示例**:
```json
{
  "success": true,
  "message": "GET 请求 /api/test 已处理",
  "task_id": 12346
}
```

## 消息格式

发送到消息队列的消息使用以下格式：

```rust
Command::HttpRequest {
    id: u64,           // 唯一任务ID
    path: String,      // 请求路径
    method: String,    // HTTP 方法 (GET/POST)
    body: Option<String>,    // 请求体内容
    headers: Option<String>, // 额外的头信息
}
```

## 使用示例

### 使用 curl 发送消息

```bash
# 发送简单消息
curl -X POST http://localhost:8080/send \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello World"}'

# 发送带数据的消息
curl -X POST http://localhost:8080/send \
  -H "Content-Type: application/json" \
  -d '{"message": "处理订单", "data": {"order_id": 123, "priority": 5}}'

# 查询状态
curl http://localhost:8080/status

# 测试通用路径
curl http://localhost:8080/api/users/123
```

### 使用 PowerShell 发送消息

```powershell
# 发送消息
$body = @{
    message = "测试消息"
    data = @{
        type = "test"
        priority = 1
    }
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:8080/send" -Method POST -Body $body -ContentType "application/json"

# 查询状态
Invoke-RestMethod -Uri "http://localhost:8080/status"
```

## 启动步骤

1. **启动守护进程**:
```bash
wsl bash -c '. ~/.cargo/env && cargo run --release --bin daemon'
```

2. **启动 Worker 进程**:
```bash
wsl bash -c '. ~/.cargo/env && cargo run --release --bin worker'
```

3. **启动 HTTP 服务器 (Entry)**:
```bash
wsl bash -c '. ~/.cargo/env && cargo run --release --bin entry'
```

## 测试

使用提供的测试脚本进行集成测试：

```powershell
# Windows PowerShell
.\scripts\test_http_integration.ps1
```

```bash
# Linux/WSL
./scripts/test_http_integration.sh
```

## 日志监控

- **Entry 日志**: `logs/entry-*.log`
- **Worker 日志**: `logs/worker-*.log`
- **Daemon 日志**: `logs/daemon-*.log`

## 错误处理

### 常见错误响应

1. **消息队列连接失败**:
```json
{
  "error": "发送消息失败: Queue connection error",
  "code": 500
}
```

2. **无效的 JSON 格式**:
```json
{
  "error": "处理请求失败: Invalid JSON format",
  "code": 500
}
```

## 性能优化

1. **日志级别控制**:
```bash
# 设置日志级别为 INFO（减少 DEBUG 输出）
export RUST_LOG=info
```

2. **队列监控**:
   - 使用 `/status` 接口监控队列状态
   - 关注 `current_size` 避免队列积压

## 扩展功能

### 自定义消息处理

在 Worker 中可以根据 `Command::HttpRequest` 的不同字段进行自定义处理：

```rust
match command {
    Command::HttpRequest { id, path, method, body, headers } => {
        match path.as_str() {
            "/api/orders" => handle_order_request(id, body),
            "/api/users" => handle_user_request(id, body),
            _ => handle_generic_request(id, path, method, body),
        }
    }
    // 其他命令类型...
}
```

### 添加认证

可以在 HTTP 服务器中添加认证中间件：

```rust
// 在 http_server.rs 中添加认证层
.layer(middleware::from_fn(auth_middleware))
```

## 故障排除

1. **端口占用**: 确保 8080 端口未被其他程序占用
2. **消息队列连接**: 确保 daemon 进程正在运行
3. **权限问题**: 确保有足够的权限创建共享内存
4. **日志查看**: 检查相应的日志文件获取详细错误信息