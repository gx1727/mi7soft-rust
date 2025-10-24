# Mi7 示例集合

这个目录包含了 Mi7 消息队列库的各种使用示例，展示了不同场景下的最佳实践。

## 示例列表

### 1. 基础队列使用 (`basic_queue_usage.rs`)
- **用途**: 演示 `SharedRingQueue` 的基本读写操作
- **特点**: 同步操作，适合理解核心概念
- **运行**: `cargo run -p examples --example basic_queue_usage`

### 2. Tokio 异步队列 (`tokio_async_queue.rs`)
- **用途**: 演示在 Tokio 异步环境中使用 `SharedRingQueue`
- **特点**: 多生产者-多消费者模式，异步协程
- **运行**: `cargo run -p examples --example tokio_async_queue`

### 3. 跨进程队列 (`cross_process_queue.rs`)
- **用途**: 演示使用 `CrossProcessQueue` 进行跨进程通信
- **特点**: 高级 API，内置线程安全，适合生产环境
- **运行**: `cargo run -p examples --example cross_process_queue`

## 编译所有示例

```bash
# 编译所有示例
cargo build -p examples

# 编译特定示例
cargo build -p examples --example basic_queue_usage

# 运行特定示例
cargo run -p examples --example cross_process_queue
```

## 示例分类

### 按复杂度分类：
1. **入门级**: `basic_queue_usage` - 理解基本概念
2. **中级**: `tokio_async_queue` - 异步编程模式
3. **高级**: `cross_process_queue` - 生产环境应用

### 按使用场景分类：
1. **单进程内通信**: `basic_queue_usage`, `tokio_async_queue`
2. **跨进程通信**: `cross_process_queue`
3. **高性能场景**: 所有示例都适用

## 最佳实践

1. **新手建议**: 从 `basic_queue_usage` 开始
2. **异步应用**: 使用 `tokio_async_queue` 模式
3. **生产环境**: 推荐 `cross_process_queue` API