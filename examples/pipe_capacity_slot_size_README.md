# CrossProcessPipe CAPACITY 和 SLOT_SIZE 参数传递指南

## 概述

`CrossProcessPipe` 是一个泛型结构体，使用编译时常量泛型参数 `CAPACITY` 和 `SLOT_SIZE` 来定义管道的容量和槽位大小。这种设计提供了零成本抽象和编译时优化。

## 核心概念

### 泛型参数定义
```rust
pub struct CrossProcessPipe<const CAPACITY: usize, const SLOT_SIZE: usize> {
    // 内部实现...
}
```

- `CAPACITY`: 管道中槽位的数量（队列深度）
- `SLOT_SIZE`: 每个槽位的字节大小（消息最大长度）

## 参数传递方式

### 1. 编译时常量泛型参数（推荐）

这是最直接和高效的方式，在编译时确定所有参数：

```rust
// 基本语法
let pipe = CrossProcessPipe::<CAPACITY, SLOT_SIZE>::create("/pipe_name")?;

// 具体示例
let small_pipe = CrossProcessPipe::<10, 1024>::create("/small")?;     // 10个槽位，每个1KB
let default_pipe = CrossProcessPipe::<100, 4096>::create("/default")?; // 100个槽位，每个4KB
let large_pipe = CrossProcessPipe::<1000, 8192>::create("/large")?;   // 1000个槽位，每个8KB
```

### 2. 类型别名简化

为常用配置创建类型别名，提高代码可读性：

```rust
// 定义类型别名
type SmallPipe = CrossProcessPipe<10, 1024>;
type DefaultPipe = CrossProcessPipe<100, 4096>;
type LargePipe = CrossProcessPipe<1000, 8192>;

// 使用别名
let pipe = DefaultPipe::create("/my_pipe")?;
```

### 3. 场景化配置

根据不同使用场景选择合适的参数：

```rust
// 控制信号管道 - 小容量，小消息
type ControlPipe = CrossProcessPipe<20, 256>;

// 数据传输管道 - 中等容量，中等消息  
type DataPipe = CrossProcessPipe<100, 4096>;

// 文件传输管道 - 小容量，大消息
type FilePipe = CrossProcessPipe<10, 65536>;

// 日志管道 - 大容量，小消息
type LogPipe = CrossProcessPipe<1000, 512>;
```

### 4. 预定义常量

使用常量提高代码维护性：

```rust
const DEFAULT_CAPACITY: usize = 100;
const DEFAULT_SLOT_SIZE: usize = 4096;

let pipe = CrossProcessPipe::<DEFAULT_CAPACITY, DEFAULT_SLOT_SIZE>::create("/pipe")?;
```

### 5. 泛型函数

创建可复用的泛型函数：

```rust
fn create_pipe<const C: usize, const S: usize>(name: &str) 
    -> Result<CrossProcessPipe<C, S>, Error> {
    CrossProcessPipe::<C, S>::create(name)
}

// 使用
let pipe = create_pipe::<100, 4096>("/my_pipe")?;
```

## 配置验证

使用 `PipeConfig` 进行运行时参数验证：

```rust
let config = PipeConfig::new(100, 4096);
let pipe = CrossProcessPipe::<100, 4096>::create_with_config("/pipe", config)?;

// 配置不匹配会返回错误
let wrong_config = PipeConfig::new(200, 8192);
let result = CrossProcessPipe::<100, 4096>::create_with_config("/pipe", wrong_config);
// 返回错误：配置不匹配
```

## 参数选择指南

### CAPACITY（容量）选择

| 场景 | 推荐容量 | 说明 |
|------|----------|------|
| 控制信号 | 10-50 | 低频率，快速响应 |
| 数据传输 | 100-500 | 中等频率，平衡性能 |
| 高频日志 | 500-2000 | 高频率，防止阻塞 |
| 批处理 | 50-200 | 批量处理，适中缓冲 |

### SLOT_SIZE（槽位大小）选择

| 数据类型 | 推荐大小 | 说明 |
|----------|----------|------|
| 控制命令 | 256-512 bytes | 简单指令 |
| JSON消息 | 1KB-4KB | 结构化数据 |
| 二进制数据 | 4KB-16KB | 序列化对象 |
| 文件块 | 16KB-64KB | 文件传输 |

## 内存使用计算

总内存使用 ≈ `CAPACITY × SLOT_SIZE + 元数据开销`

```rust
// 示例计算
let small_memory = 10 * 1024;      // ~10KB
let default_memory = 100 * 4096;   // ~400KB  
let large_memory = 1000 * 8192;    // ~8MB
```

## 最佳实践

### 1. 根据消息大小选择槽位
```rust
// 小消息高频
type EventPipe = CrossProcessPipe<1000, 512>;

// 大消息低频  
type FilePipe = CrossProcessPipe<10, 65536>;
```

### 2. 使用有意义的类型别名
```rust
type LoggerPipe = CrossProcessPipe<500, 1024>;
type MetricsPipe = CrossProcessPipe<200, 2048>;
type CommandPipe = CrossProcessPipe<50, 256>;
```

### 3. 配置常量集中管理
```rust
mod pipe_configs {
    pub const LOG_CAPACITY: usize = 500;
    pub const LOG_SLOT_SIZE: usize = 1024;
    
    pub const METRIC_CAPACITY: usize = 200;
    pub const METRIC_SLOT_SIZE: usize = 2048;
}
```

### 4. 性能考虑
- 较大的 `CAPACITY` 提供更好的缓冲，但占用更多内存
- 较大的 `SLOT_SIZE` 支持更大消息，但可能浪费空间
- 选择 2 的幂次方大小通常有更好的内存对齐

## 常见错误

### 1. 运行时传参
```rust
// ❌ 错误：不能在运行时传递泛型参数
fn create_dynamic_pipe(capacity: usize, slot_size: usize) {
    // 这是不可能的，因为泛型参数必须在编译时确定
}
```

### 2. 配置不匹配
```rust
// ❌ 错误：配置与泛型参数不匹配
let config = PipeConfig::new(200, 8192);
let pipe = CrossProcessPipe::<100, 4096>::create_with_config("/pipe", config)?;
// 会返回配置不匹配错误
```

### 3. 过大的参数
```rust
// ❌ 可能有问题：过大的内存使用
type HugePipe = CrossProcessPipe<10000, 1048576>; // ~10GB 内存
```

## 运行示例

```bash
# 编译示例
cargo build --bin pipe_capacity_slot_size_example

# 运行示例
cargo run --bin pipe_capacity_slot_size_example
```

## 总结

- `CAPACITY` 和 `SLOT_SIZE` 是编译时常量泛型参数
- 必须在类型声明时指定具体数值
- 使用类型别名可以简化重复的泛型参数声明
- 根据具体使用场景选择合适的容量和槽位大小
- 配置对象主要用于运行时验证，而非参数传递
- 合理的参数选择对性能和内存使用至关重要