# 配置化队列使用指南

## 概述

MI7 队列系统现在支持配置化的队列大小和槽位大小，允许您根据具体需求调整队列参数以获得最佳性能。

## 核心组件

### QueueConfig 结构体

```rust
#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    pub capacity: usize,    // 队列槽位数量
    pub slot_size: usize,   // 每个槽位的大小（字节）
}
```

### 预定义配置

```rust
// 默认配置：100个槽位，每个4KB
let config = QueueConfig::default();

// 小型配置：10个槽位，每个1KB
let config = QueueConfig::small();

// 大型配置：1000个槽位，每个8KB
let config = QueueConfig::large();

// 自定义配置
let config = QueueConfig::new(50, 2048);
```

## 使用方式

### 1. 类型别名方式（推荐）

```rust
use mi7::{DefaultCrossProcessQueue, SmallCrossProcessQueue, LargeCrossProcessQueue};

// 使用默认配置
let queue = DefaultCrossProcessQueue::create_default("my_queue")?;

// 使用小型配置
let small_queue = SmallCrossProcessQueue::create("small_queue")?;

// 使用大型配置
let large_queue = LargeCrossProcessQueue::create("large_queue")?;
```

### 2. 泛型方式

```rust
use mi7::CrossProcessQueue;

// 自定义配置：50个槽位，每个2KB
type MyQueue = CrossProcessQueue<50, 2048>;
let queue = MyQueue::create("custom_queue")?;
```

### 3. 配置验证方式

```rust
use mi7::{DefaultCrossProcessQueue, QueueConfig};

let config = QueueConfig::new(100, 4096);
let queue = DefaultCrossProcessQueue::create_with_config("validated_queue", config)?;
```

## 配置选择指南

### 小型队列 (10槽位, 1KB)
- **适用场景**: 轻量级通信、控制信号传递
- **优势**: 内存占用小，启动快
- **限制**: 容量有限，不适合大数据传输

```rust
let queue = SmallCrossProcessQueue::create("control_queue")?;
```

### 默认队列 (100槽位, 4KB)
- **适用场景**: 一般应用场景、中等负载
- **优势**: 平衡的性能和内存使用
- **推荐**: 大多数应用的首选

```rust
let queue = DefaultCrossProcessQueue::create_default("app_queue")?;
```

### 大型队列 (1000槽位, 8KB)
- **适用场景**: 高吞吐量、大数据传输
- **优势**: 高容量，支持大消息
- **注意**: 内存占用较大

```rust
let queue = LargeCrossProcessQueue::create("data_pipeline")?;
```

### 自定义配置
- **适用场景**: 特殊需求、性能优化
- **灵活性**: 完全自定义参数

```rust
// 高频小消息场景
type HighFreqQueue = CrossProcessQueue<500, 512>;

// 低频大消息场景  
type LowFreqQueue = CrossProcessQueue<20, 16384>;
```

## 性能考虑

### 槽位数量 (Capacity)
- **更多槽位**: 更高并发，更少阻塞
- **更少槽位**: 更低内存占用，更快启动

### 槽位大小 (Slot Size)
- **更大槽位**: 支持更大消息，减少分片
- **更小槽位**: 更低内存占用，更好缓存局部性

### 内存使用计算
```
总内存使用 ≈ 槽位数量 × 槽位大小 + 元数据开销
```

## 最佳实践

### 1. 根据消息大小选择槽位大小
```rust
// 小消息 (<1KB)
type SmallMsgQueue = CrossProcessQueue<200, 1024>;

// 中等消息 (1-4KB)  
type MediumMsgQueue = CrossProcessQueue<100, 4096>;

// 大消息 (4-16KB)
type LargeMsgQueue = CrossProcessQueue<50, 16384>;
```

### 2. 根据并发需求选择槽位数量
```rust
// 低并发
type LowConcurrencyQueue = CrossProcessQueue<10, 4096>;

// 中等并发
type MediumConcurrencyQueue = CrossProcessQueue<100, 4096>;

// 高并发
type HighConcurrencyQueue = CrossProcessQueue<1000, 4096>;
```

### 3. 监控和调优
```rust
let queue = DefaultCrossProcessQueue::create("monitored_queue")?;

// 获取配置信息
let config = queue.config();
println!("队列配置: {:?}", config);

// 获取状态信息
let status = queue.status();
println!("队列状态: 容量={}, 消息数={}", status.capacity, status.message_count);
```

## 向后兼容性

现有代码可以无缝迁移到新的配置化API：

```rust
// 旧代码
use mi7::CrossProcessQueue;
let queue = CrossProcessQueue::create("old_queue")?;

// 新代码（等价）
use mi7::DefaultCrossProcessQueue;
let queue = DefaultCrossProcessQueue::create("new_queue")?;
```

## 示例代码

完整的使用示例请参考：
- `examples/configurable_queue_test.rs` - 配置化功能演示
- `examples/cross_process_queue.rs` - 异步集成示例

## 注意事项

1. **编译时确定**: 队列配置在编译时确定，运行时无法更改
2. **兼容性**: 不同配置的队列无法互相通信
3. **内存对齐**: 槽位大小建议使用2的幂次方以获得更好性能
4. **系统限制**: 受操作系统共享内存限制影响

## 故障排除

### 配置不匹配错误
```
配置不匹配：期望 capacity=100, slot_size=4096，实际 capacity=200, slot_size=8192
```
**解决方案**: 确保 `create_with_config` 中的配置与类型参数匹配

### 内存不足错误
```
Failed to create shared ring queue
```
**解决方案**: 减少槽位数量或槽位大小，或增加系统共享内存限制