# 跨进程消息队列使用指南

## 概述

本项目实现了一个基于共享内存的跨进程消息队列，支持多个生产者和消费者之间的高效通信。

## 核心特性

### 🚀 主要功能
- **跨进程通信**: 基于命名共享内存实现真正的跨进程通信
- **多生产者/多消费者**: 支持多个进程同时发送和接收消息
- **线程安全**: 使用原子操作和自旋锁确保并发安全
- **高性能**: 零拷贝的共享内存访问，最小化序列化开销
- **可靠性**: 内置消息完整性检查和错误处理

### 🔧 技术实现
- **共享内存**: 使用 `shared_memory` crate 创建命名共享内存段
- **同步机制**: 自实现的自旋锁保证原子操作
- **序列化**: 使用 `bincode` 进行高效的消息序列化
- **内存布局**: 优化的环形缓冲区设计

## 使用方法

### 1. 基本用法

#### 生产者 (Entry)
```rust
use mi7soft::ipc_queue::{CrossProcessQueue, Message};

// 创建消息队列
let queue = CrossProcessQueue::create("task_queue", 100, 1024)?;

// 发送消息
let message = Message {
    id: 1,
    data: "Hello World".as_bytes().to_vec(),
    timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
};

queue.send(&message)?;
```

#### 消费者 (Worker)
```rust
use mi7soft::ipc_queue::CrossProcessQueue;

// 连接到已存在的队列
let queue = CrossProcessQueue::connect("task_queue")?;

// 接收消息
while let Some(message) = queue.receive()? {
    println!("收到消息: {}", String::from_utf8_lossy(&message.data));
    // 处理消息...
}
```

### 2. 运行示例

#### 方式一：手动运行
```bash
# 终端1: 启动生产者
wsl bash -c '. ~/.cargo/env && cargo run --example producer'

# 终端2: 启动消费者1
wsl bash -c '. ~/.cargo/env && cargo run --example worker worker-1'

# 终端3: 启动消费者2
wsl bash -c '. ~/.cargo/env && cargo run --example worker worker-2'
```

#### 方式二：使用演示脚本
```powershell
# 运行完整演示（自动启动多个进程）
.\run_demo.ps1
```

## 演示结果

### 成功运行的输出示例

**生产者输出:**
```
🚀 启动消息生产者 (Entry)
📝 开始发送任务消息...
✅ 发送任务 1: Task 1 - Process this data
📊 队列状态: 1/100 消息
✅ 发送任务 2: Task 2 - Process this data
📊 队列状态: 2/100 消息
...
🏁 生产者完成，发送了 20 个任务
```

**消费者输出:**
```
🔧 启动 Worker worker-1 (PID: 2279341)
📡 Worker worker-1 已连接到任务队列
🔄 Worker worker-1 处理任务 1: Task 1 - Process this data
✅ Worker worker-1 完成任务 1 (耗时: 300ms)
📊 Worker worker-1 队列状态: 19/100 消息剩余
...
```

## 架构设计

### 内存布局
```
共享内存段:
┌─────────────────┬──────────────────────────────────┐
│   QueueHeader   │           Message Data           │
│   (元数据区)     │          (消息存储区)             │
└─────────────────┴──────────────────────────────────┘
```

### QueueHeader 结构
- `magic`: 魔数，用于验证内存完整性
- `read_pos`: 读取位置（原子操作）
- `write_pos`: 写入位置（原子操作）
- `message_count`: 当前消息数量
- `max_messages`: 最大消息数量
- `message_size`: 每个消息的最大大小
- `writer_lock`: 写入锁
- `reader_lock`: 读取锁

### 同步机制
1. **写入锁**: 确保同时只有一个生产者写入
2. **读取锁**: 确保同时只有一个消费者读取
3. **原子计数器**: 跟踪消息数量和位置
4. **自旋锁**: 高性能的锁实现，适合短时间持有

## 性能特点

### 优势
- **零拷贝**: 消息直接在共享内存中传递
- **低延迟**: 避免了系统调用和网络开销
- **高吞吐**: 支持批量消息处理
- **内存效率**: 环形缓冲区重复利用内存

### 适用场景
- 同机器多进程通信
- 高频率消息传递
- 低延迟要求的应用
- 大数据量的进程间传输

## 错误处理

### 常见错误类型
- `CreationFailed`: 共享内存创建失败
- `AccessFailed`: 共享内存访问失败
- `QueueFull`: 队列已满
- `QueueEmpty`: 队列为空
- `SerializationFailed`: 消息序列化失败
- `DeserializationFailed`: 消息反序列化失败
- `CorruptedData`: 数据损坏

### 错误处理建议
```rust
match queue.send(&message) {
    Ok(()) => println!("消息发送成功"),
    Err(SharedMemoryError::QueueFull) => {
        // 队列满，等待或丢弃消息
        thread::sleep(Duration::from_millis(10));
    }
    Err(e) => eprintln!("发送失败: {}", e),
}
```

## 注意事项

### 1. 内存管理
- 共享内存在所有进程退出后才会被清理
- 建议在程序退出时显式清理资源

### 2. 并发控制
- 自旋锁适合短时间持有，避免长时间阻塞
- 消息处理应该尽快完成，避免影响其他消费者

### 3. 消息大小
- 消息大小不能超过预设的 `message_size`
- 建议根据实际需求合理设置消息大小

### 4. 进程生命周期
- 生产者应该在消费者之前启动
- 消费者可以在任何时候连接到已存在的队列

## 扩展功能

### 可能的改进方向
1. **持久化**: 支持消息持久化到磁盘
2. **优先级**: 支持消息优先级队列
3. **分区**: 支持多个独立的消息分区
4. **监控**: 添加详细的性能监控和统计
5. **压缩**: 支持消息压缩以节省内存

## 总结

这个跨进程消息队列实现展示了如何使用 Rust 和共享内存技术构建高性能的进程间通信系统。它提供了：

✅ **完整的跨进程通信解决方案**  
✅ **高性能的共享内存实现**  
✅ **线程安全的并发控制**  
✅ **易于使用的 API 接口**  
✅ **详细的错误处理机制**  

这个实现可以作为构建更复杂分布式系统的基础，也可以直接用于需要高性能进程间通信的应用场景。