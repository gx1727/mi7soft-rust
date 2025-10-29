# AsyncFutex::wait_async 使用指南

## 概述

`wait_async` 是 `AsyncFutex` 的核心异步方法，用于在 Tokio 异步环境中等待共享内存中的原子变量值发生变化。它结合了 Linux futex 系统调用和 Tokio 的异步 I/O 机制，提供高效的跨进程同步。

## 方法签名

```rust
pub async fn wait_async(&self, expected: u32) -> std::io::Result<()>
```

### 参数说明
- `expected: u32` - 期望的当前值。如果共享变量的值等于这个值，线程将进入等待状态
- 返回值：`std::io::Result<()>` - 成功时返回 `Ok(())`，失败时返回 I/O 错误

## 工作原理

1. **值检查**：首先检查共享内存中的原子变量值是否等于 `expected`
2. **Futex 等待**：如果值相等，调用 Linux futex 系统调用进入等待状态
3. **异步监听**：通过 eventfd 和 Tokio 的 `AsyncFd` 实现异步唤醒机制
4. **唤醒处理**：当其他进程调用 `wake()` 时，等待的任务会被唤醒

## 基本使用模式

### 1. 简单等待模式
```rust
use std::sync::atomic::{AtomicU32, Ordering};
use mi7::AsyncFutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建共享原子变量
    let shared_value = Box::leak(Box::new(AtomicU32::new(0)));
    let futex = AsyncFutex::new(shared_value)?;
    
    // 等待值变为非0
    println!("等待值变化...");
    futex.wait_async(0).await?;
    println!("值已变化！");
    
    Ok(())
}
```

### 2. 状态机等待模式
```rust
const STATE_IDLE: u32 = 0;
const STATE_WORKING: u32 = 1;
const STATE_DONE: u32 = 2;

async fn wait_for_work_completion(
    futex: &AsyncFutex,
    state: &AtomicU32
) -> std::io::Result<()> {
    loop {
        let current_state = state.load(Ordering::SeqCst);
        match current_state {
            STATE_DONE => break,
            _ => {
                // 等待状态变化
                futex.wait_async(current_state).await?;
            }
        }
    }
    Ok(())
}
```

### 3. 生产者-消费者模式
```rust
// 消费者等待新数据
async fn consumer_wait(
    futex: &AsyncFutex,
    data_counter: &AtomicU32
) -> std::io::Result<()> {
    let last_seen = data_counter.load(Ordering::SeqCst);
    
    // 等待新数据到达
    futex.wait_async(last_seen).await?;
    
    // 处理新数据
    let new_count = data_counter.load(Ordering::SeqCst);
    println!("处理 {} 个新数据项", new_count - last_seen);
    
    Ok(())
}
```

## 高级使用场景

### 1. 超时等待
```rust
use tokio::time::{timeout, Duration};

async fn wait_with_timeout(
    futex: &AsyncFutex,
    expected: u32
) -> Result<(), Box<dyn std::error::Error>> {
    match timeout(Duration::from_secs(5), futex.wait_async(expected)).await {
        Ok(result) => {
            result?;
            println!("等待成功完成");
        }
        Err(_) => {
            println!("等待超时");
        }
    }
    Ok(())
}
```

### 2. 多条件等待
```rust
use tokio::select;

async fn wait_multiple_conditions(
    futex1: &AsyncFutex,
    futex2: &AsyncFutex,
    condition1: u32,
    condition2: u32
) -> Result<String, Box<dyn std::error::Error>> {
    select! {
        result1 = futex1.wait_async(condition1) => {
            result1?;
            Ok("条件1满足".to_string())
        }
        result2 = futex2.wait_async(condition2) => {
            result2?;
            Ok("条件2满足".to_string())
        }
    }
}
```

### 3. 循环等待模式
```rust
async fn event_loop(
    futex: &AsyncFutex,
    event_counter: &AtomicU32
) -> std::io::Result<()> {
    let mut last_event = 0u32;
    
    loop {
        // 等待新事件
        futex.wait_async(last_event).await?;
        
        // 处理所有新事件
        let current_events = event_counter.load(Ordering::SeqCst);
        for event_id in (last_event + 1)..=current_events {
            println!("处理事件 {}", event_id);
        }
        
        last_event = current_events;
    }
}
```

## 错误处理

### 常见错误类型
```rust
async fn handle_wait_errors(
    futex: &AsyncFutex,
    expected: u32
) -> Result<(), Box<dyn std::error::Error>> {
    match futex.wait_async(expected).await {
        Ok(()) => {
            println!("等待成功完成");
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            println!("等待被中断，重试...");
            // 可以选择重试
        }
        Err(e) => {
            eprintln!("等待失败: {}", e);
            return Err(e.into());
        }
    }
    Ok(())
}
```

## 性能优化建议

### 1. 避免忙等待
```rust
// ❌ 错误：忙等待
loop {
    if shared_value.load(Ordering::SeqCst) != expected {
        break;
    }
    tokio::task::yield_now().await;
}

// ✅ 正确：使用 wait_async
futex.wait_async(expected).await?;
```

### 2. 合理选择内存序
```rust
// 在检查值之前使用适当的内存序
let current_value = shared_value.load(Ordering::Acquire);
if current_value == expected {
    futex.wait_async(expected).await?;
}
```

### 3. 批量处理
```rust
// 批量处理多个等待的变化
async fn batch_wait(
    futex: &AsyncFutex,
    counters: &[&AtomicU32]
) -> std::io::Result<()> {
    let initial_sum: u32 = counters.iter()
        .map(|c| c.load(Ordering::SeqCst))
        .sum();
    
    futex.wait_async(initial_sum).await?;
    
    // 处理所有变化
    for counter in counters {
        let value = counter.load(Ordering::SeqCst);
        println!("计数器值: {}", value);
    }
    
    Ok(())
}
```

## 注意事项

1. **运行时要求**：必须在 Tokio 异步运行时中使用
2. **平台限制**：仅支持 Linux 系统（使用 futex 系统调用）
3. **内存安全**：确保共享内存的生命周期管理正确
4. **竞态条件**：在检查值和调用 `wait_async` 之间可能发生竞态条件
5. **唤醒丢失**：如果在调用 `wait_async` 之前值已经改变，可能错过唤醒信号

## 与其他同步原语的比较

| 特性 | AsyncFutex | tokio::sync::Notify | std::sync::Condvar |
|------|------------|---------------------|-------------------|
| 跨进程 | ✅ | ❌ | ❌ |
| 异步 | ✅ | ✅ | ❌ |
| 零拷贝 | ✅ | ❌ | ❌ |
| 内核支持 | ✅ | ❌ | ✅ |
| 平台支持 | Linux only | 跨平台 | 跨平台 |

## 实际项目中的使用

在 mi7 项目中，`wait_async` 主要用于：
- 跨进程管道的槽位状态同步
- 生产者-消费者队列的协调
- 分布式任务的状态监控
- 资源锁的异步等待

通过合理使用 `wait_async`，可以实现高效的跨进程异步同步，避免轮询带来的 CPU 浪费。