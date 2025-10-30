# MI7 管道配置使用指南

## 概述

MI7 提供了灵活的管道配置系统，支持不同规模的队列配置以满足各种使用场景。本指南详细介绍如何使用小型队列配置和大型队列配置。

## 配置类型

### 1. 预定义配置

MI7 提供了三种预定义的配置：

```rust
// 默认配置：100个槽位，每个4KB
pub fn default() -> Self {
    Self {
        capacity: 100,
        slot_size: 4096,
    }
}

// 小型队列配置：10个槽位，每个1KB
pub fn small() -> Self {
    Self {
        capacity: 10,
        slot_size: 1024,
    }
}

// 大型队列配置：1000个槽位，每个8KB
pub fn large() -> Self {
    Self {
        capacity: 1000,
        slot_size: 8192,
    }
}
```

### 2. 类型别名

为了方便使用，MI7 提供了对应的类型别名：

```rust
// 默认配置的类型别名
pub type DefaultCrossProcessPipe = CrossProcessPipe<100, 4096>;

// 小型配置的类型别名
pub type SmallCrossProcessPipe = CrossProcessPipe<10, 1024>;

// 大型配置的类型别名
pub type LargeCrossProcessPipe = CrossProcessPipe<1000, 8192>;
```

## 使用方式

### 方式 1：使用类型别名（推荐）

#### 小型队列配置

```rust
use mi7::pipe::SmallCrossProcessPipe;
use mi7::{Message, shared_slot::SlotState};

fn use_small_pipe() -> Result<(), Box<dyn std::error::Error>> {
    // 创建小型队列
    let pipe = SmallCrossProcessPipe::create("small_queue")?;

    println!("小型队列配置:");
    println!("  容量: {} 槽位", pipe.capacity());
    println!("  槽位大小: {} bytes", pipe.slot_size());
    println!("  总内存: ~{} KB", (pipe.capacity() * pipe.slot_size()) / 1024);

    // 基本操作
    let slot_index = pipe.hold()?;
    pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;

    let message = Message::new(1, "小型队列测试消息".to_string());
    let request_id = pipe.send(slot_index, message)?;

    println!("发送消息成功，请求ID: {}", request_id);

    Ok(())
}
```

#### 大型队列配置

```rust
use mi7::pipe::LargeCrossProcessPipe;
use mi7::{Message, shared_slot::SlotState};

fn use_large_pipe() -> Result<(), Box<dyn std::error::Error>> {
    // 创建大型队列
    let pipe = LargeCrossProcessPipe::create("large_queue")?;

    println!("大型队列配置:");
    println!("  容量: {} 槽位", pipe.capacity());
    println!("  槽位大小: {} bytes", pipe.slot_size());
    println!("  总内存: ~{} MB", (pipe.capacity() * pipe.slot_size()) / (1024 * 1024));

    // 基本操作
    let slot_index = pipe.hold()?;
    pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;

    let message = Message::new(1, "大型队列测试消息".to_string());
    let request_id = pipe.send(slot_index, message)?;

    println!("发送消息成功，请求ID: {}", request_id);

    Ok(())
}
```

### 方式 2：使用配置结构体

```rust
use mi7::pipe::{PipeConfig, CrossProcessPipe};

fn use_config_struct() -> Result<(), Box<dyn std::error::Error>> {
    // 获取预定义配置
    let small_config = PipeConfig::small();
    let large_config = PipeConfig::large();

    println!("配置信息:");
    println!("  小型配置: {:?}", small_config);
    println!("  大型配置: {:?}", large_config);

    // 自定义配置
    let custom_config = PipeConfig::new(50, 2048);
    println!("  自定义配置: {:?}", custom_config);

    // 注意：由于泛型参数在编译时确定，配置验证主要用于运行时检查
    // 实际创建时仍需要使用对应的类型别名或泛型参数

    Ok(())
}
```

### 方式 3：直接使用泛型参数

```rust
use mi7::pipe::CrossProcessPipe;

fn use_generic_params() -> Result<(), Box<dyn std::error::Error>> {
    // 直接指定泛型参数
    let small_pipe = CrossProcessPipe::<10, 1024>::create("generic_small")?;
    let large_pipe = CrossProcessPipe::<1000, 8192>::create("generic_large")?;

    println!("泛型参数方式:");
    println!("  小型: {}槽位 x {}bytes", small_pipe.capacity(), small_pipe.slot_size());
    println!("  大型: {}槽位 x {}bytes", large_pipe.capacity(), large_pipe.slot_size());

    Ok(())
}
```

## 使用场景

### 小型队列配置 (10 槽位 x 1KB)

**适用场景：**

- 轻量级消息传递
- 低并发环境
- 内存受限的系统
- 简单的控制信号传递
- 测试和原型开发

**优势：**

- 内存占用小 (~10KB)
- 启动速度快
- 缓存友好
- 适合嵌入式系统

**示例用例：**

```rust
// 系统状态监控
let monitor_pipe = SmallCrossProcessPipe::create("system_monitor")?;

// 配置更新通知
let config_pipe = SmallCrossProcessPipe::create("config_updates")?;

// 简单的任务调度
let task_pipe = SmallCrossProcessPipe::create("simple_tasks")?;
```

### 大型队列配置 (1000 槽位 x 8KB)

**适用场景：**

- 高并发消息处理
- 大数据传输
- 批处理系统
- 实时数据流
- 企业级应用

**优势：**

- 高吞吐量
- 支持大消息
- 减少阻塞概率
- 适合生产环境

**示例用例：**

```rust
// 高频交易数据
let trading_pipe = LargeCrossProcessPipe::create("trading_data")?;

// 日志收集系统
let log_pipe = LargeCrossProcessPipe::create("log_collector")?;

// 图像处理管道
let image_pipe = LargeCrossProcessPipe::create("image_processor")?;
```

## 性能对比

| 配置类型 | 槽位数量 | 槽位大小 | 总内存 | 适用并发度 | 消息大小限制 |
| -------- | -------- | -------- | ------ | ---------- | ------------ |
| 小型     | 10       | 1KB      | ~10KB  | 低         | <1KB         |
| 默认     | 100      | 4KB      | ~400KB | 中等       | <4KB         |
| 大型     | 1000     | 8KB      | ~8MB   | 高         | <8KB         |

## 最佳实践

### 1. 根据消息大小选择配置

```rust
// 小消息 (<1KB) - 使用小型配置
let small_msg_pipe = SmallCrossProcessPipe::create("small_messages")?;

// 中等消息 (1-4KB) - 使用默认配置
let medium_msg_pipe = DefaultCrossProcessPipe::create("medium_messages")?;

// 大消息 (4-8KB) - 使用大型配置
let large_msg_pipe = LargeCrossProcessPipe::create("large_messages")?;
```

### 2. 根据并发需求选择配置

```rust
// 低并发 - 小型配置
let low_concurrency = SmallCrossProcessPipe::create("low_traffic")?;

// 高并发 - 大型配置
let high_concurrency = LargeCrossProcessPipe::create("high_traffic")?;
```

### 3. 监控和调优

```rust
fn monitor_pipe_performance<const CAPACITY: usize, const SLOT_SIZE: usize>(
    pipe: &CrossProcessPipe<CAPACITY, SLOT_SIZE>,
    name: &str,
) {
    let status = pipe.status();
    let config = pipe.config();

    println!("=== {} 性能监控 ===", name);
    println!("配置: {:?}", config);
    println!("状态: {:?}", status);

    // 计算使用率
    let usage_rate = (status.used_count as f64 / status.capacity as f64) * 100.0;
    println!("使用率: {:.1}%", usage_rate);

    // 内存使用
    let memory_usage = (status.capacity * status.slot_size) / 1024;
    println!("内存使用: {} KB", memory_usage);

    // 性能建议
    if usage_rate > 80.0 {
        println!("⚠️  建议：队列使用率过高，考虑增加容量");
    }
    if status.ready_count > status.capacity / 2 {
        println!("⚠️  建议：消费速度可能跟不上生产速度");
    }
}
```

## 连接到现有队列

```rust
// 连接到小型队列
let small_consumer = SmallCrossProcessPipe::connect("small_queue")?;

// 连接到大型队列
let large_consumer = LargeCrossProcessPipe::connect("large_queue")?;
```

## 错误处理

```rust
use mi7::pipe::{SmallCrossProcessPipe, LargeCrossProcessPipe};

fn robust_pipe_usage() -> Result<(), Box<dyn std::error::Error>> {
    // 尝试创建小型队列
    let small_pipe = match SmallCrossProcessPipe::create("robust_small") {
        Ok(pipe) => {
            println!("✅ 小型队列创建成功");
            pipe
        }
        Err(e) => {
            eprintln!("❌ 小型队列创建失败: {}", e);
            return Err(e);
        }
    };

    // 尝试创建大型队列
    let large_pipe = match LargeCrossProcessPipe::create("robust_large") {
        Ok(pipe) => {
            println!("✅ 大型队列创建成功");
            pipe
        }
        Err(e) => {
            eprintln!("❌ 大型队列创建失败: {}", e);
            return Err(e);
        }
    };

    // 使用队列...

    Ok(())
}
```

## 总结

MI7 的管道配置系统提供了灵活的选择：

1. **小型配置**：适合轻量级、低并发场景
2. **大型配置**：适合高并发、大数据场景
3. **类型别名**：提供编译时类型安全
4. **配置结构体**：提供运行时配置信息

选择合适的配置可以显著提升系统性能和资源利用率。建议根据实际的消息大小、并发需求和内存限制来选择最适合的配置。
