# Mi7Soft - 高性能跨进程消息队列库

一个基于共享内存的高性能跨进程消息队列库，使用 Rust 实现，支持异步操作和智能等待策略。

## 特性

- **高性能**: 基于共享内存的零拷贝消息传递
- **异步支持**: 完整的 Tokio 异步运行时支持
- **线程安全**: 使用智能锁机制确保并发安全
- **智能等待**: 避免自旋锁，使用异步等待策略
- **大数据支持**: 支持大型消息的高效传输
- **跨进程**: 支持多进程间的消息队列通信
- **状态监控**: 实时队列状态和性能监控

## 依赖项

```toml
[dependencies]
memmap2 = "0.9"           # 内存映射文件支持
tokio = { version = "1.0", features = ["full"] }  # 异步运行时
bincode = "2.0"           # 高效序列化
serde = { version = "1.0", features = ["derive"] }  # 序列化框架
```

## 快速开始

### 安装和编译

```bash
# 克隆项目
git clone <repository-url>
cd mi7soft-rust

# 编译项目
wsl bash -c '. ~/.cargo/env && cargo build --release'
```

### 基本使用示例

#### 1. 消息生产者

```rust
use mi7::{CrossProcessQueue, Message};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建消息队列
    let queue = CrossProcessQueue::create("task_queue")?;
    
    // 发送消息
    let message = Message::new("Hello, World!".to_string());
    
    queue.send(message)?;
    println!("消息发送成功！");
    
    Ok(())
}
```

#### 2. 消息消费者

```rust
use mi7::CrossProcessQueue;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到消息队列
    let queue = CrossProcessQueue::connect("task_queue")?;
    
    // 接收消息
    loop {
        match queue.try_receive()? {
            Some(message) => {
                println!("收到消息 {}: {}", 
                         message.id, 
                         String::from_utf8_lossy(&message.data));
                
                // 处理消息...
            }
            None => {
                println!("队列为空，等待新消息...");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
    
    Ok(())
}
```

## 运行示例

### 启动消息生产者

```bash
# 编译并运行生产者
wsl bash -c '. ~/.cargo/env && cargo run --bin entry'
```

### 启动消息消费者

```bash
# 编译并运行消费者（可以启动多个）
wsl bash -c '. ~/.cargo/env && cargo run --bin worker'

# 启动多个 worker 处理消息
wsl bash -c '. ~/.cargo/env && cargo run --bin worker worker1'
wsl bash -c '. ~/.cargo/env && cargo run --bin worker worker2'
```

## 项目结构

```
mi7soft-rust/
├── Cargo.toml              # 工作空间配置
├── README.md               # 项目文档
├── mi7/                    # 核心库
│   ├── src/
│   │   ├── lib.rs          # 库入口
│   │   ├── shared_ring.rs  # 共享环形队列实现
│   │   └── queue.rs        # 跨进程消息队列包装器
│   └── Cargo.toml
├── daemon/                 # 守护进程
├── entry/                  # 入口程序
└── worker/                 # 工作进程
```

## 核心 API

### CrossProcessQueue

主要的消息队列类，提供以下方法：

```rust
impl CrossProcessQueue {
    // 创建新的消息队列
    pub fn create(name: &str, max_messages: usize, max_message_size: usize) -> Result<Self>;
    
    // 连接到现有的消息队列
    pub fn connect(name: &str) -> Result<Self>;
    
    // 发送消息（同步）
    pub fn send(&self, message: &Message) -> Result<()>;
    
    // 接收消息（同步）
    pub fn receive(&self) -> Result<Option<Message>>;
    
    // 异步接收消息（带超时）
    pub async fn receive_async_with_timeout(&self, timeout: Duration) -> Result<Option<Message>>;
    
    // 获取队列状态
    pub fn status(&self) -> QueueStatus;
}
```

### Message

消息结构体：

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub data: Vec<u8>,
    pub timestamp: u64,
}
```

## 性能特点

### 高性能设计

- **零拷贝传输**: 基于共享内存，避免数据复制
- **智能锁机制**: 使用自旋锁 + yield 策略，减少上下文切换
- **异步等待**: 避免忙等待，使用 `tokio::time::sleep` 进行智能等待
- **批量处理**: 支持高吞吐量的消息处理

### 性能指标（参考）

- **延迟**: 微秒级消息传递延迟
- **吞吐量**: 支持每秒数万条消息
- **内存效率**: 固定大小的共享内存池
- **CPU 使用**: 智能等待策略，低 CPU 占用

## 监控和调试

### 队列状态监控

```rust
let status = queue.status();
println!("队列状态:");
println!("  消息数量: {}/{}", status.message_count, status.max_messages);
println!("  队列使用率: {:.1}%", 
         (status.message_count as f64 / status.max_messages as f64) * 100.0);
```

### 错误处理

库提供了详细的错误类型：

```rust
pub enum SharedMemoryError {
    CreationFailed(String),
    AccessFailed(String),
    LockFailed(String),
    QueueFull,
    QueueEmpty,
    SerializationFailed(String),
    // ... 更多错误类型
}
```

## 使用场景

- **微服务通信**: 高性能的服务间消息传递
- **任务队列**: 分布式任务处理系统
- **实时数据流**: 低延迟的数据流处理
- **批处理系统**: 大批量数据处理管道
- **游戏服务器**: 实时游戏状态同步

## 配置选项

### 队列参数

- `max_messages`: 队列最大消息数量
- `max_message_size`: 单个消息最大大小
- `timeout`: 异步接收超时时间

### 性能调优

- 根据消息大小调整 `max_message_size`
- 根据并发量调整 `max_messages`
- 使用适当的超时时间避免资源浪费

## 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 致谢

- [tokio](https://crates.io/crates/tokio) - 异步运行时
- [serde](https://crates.io/crates/serde) - 序列化框架
- [bincode](https://crates.io/crates/bincode) - 高效二进制序列化
- [libc](https://crates.io/crates/libc) - 系统调用接口

---

如果这个项目对您有帮助，请给它一个星标！

如有问题或建议，请创建 Issue 或发送 Pull Request。