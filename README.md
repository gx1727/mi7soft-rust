# Rust 共享内存和锁机制 Demo

🚀 一个全面的Rust共享内存和锁机制演示项目，展示了多种并发编程技术和性能优化方案。

## 📋 项目概述

本项目实现了多种共享内存方案和锁机制，包括：

### 🧠 共享内存实现
- **mmap共享内存**: 基于内存映射的高性能共享内存
- **跨进程共享内存**: 使用`shared_memory` crate实现的进程间通信
- **共享内存管理器**: 统一管理多个共享内存实例

### 🔒 锁机制实现
- **标准库Mutex**: 带性能统计的Mutex包装器
- **标准库RwLock**: 支持读写分离的锁机制
- **Parking Lot锁**: 高性能的第三方锁实现
- **原子操作**: 无锁的原子计数器
- **自旋锁**: 自定义实现的自旋锁机制

### 🎯 特色功能
- **性能统计**: 所有锁都包含详细的性能统计信息
- **多线程支持**: 完整的多线程并发示例
- **异步支持**: Tokio异步编程示例
- **并行计算**: Rayon并行计算集成
- **基准测试**: 使用Criterion的详细性能测试
- **系统监控**: 内存使用和系统信息监控

## 🛠️ 依赖项

```toml
[dependencies]
shared_memory = "0.12"    # 跨进程共享内存
sysinfo = "0.30"          # 系统信息获取
libc = "0.2"              # 系统调用接口
memmap2 = "0.9"           # 内存映射
parking_lot = "0.12"      # 高性能锁
crossbeam = "0.8"         # 并发工具
tokio = "1.0"             # 异步运行时
rayon = "1.8"             # 并行计算
criterion = "0.5"         # 基准测试
```

## 🚀 快速开始

### 安装和编译

```bash
# 克隆项目
git clone <repository-url>
cd mi7soft-rust

# 编译项目
wsl bash -c '. ~/.cargo/env && cargo build --release'

# 运行所有示例
wsl bash -c '. ~/.cargo/env && cargo run'
```

### 运行特定示例

```bash
# 多线程共享内存示例
wsl bash -c '. ~/.cargo/env && cargo run multithreaded'

# 锁性能比较
wsl bash -c '. ~/.cargo/env && cargo run performance'

# 读写锁示例
wsl bash -c '. ~/.cargo/env && cargo run rwlock'

# Rayon并行计算
wsl bash -c '. ~/.cargo/env && cargo run rayon'

# Tokio异步示例
wsl bash -c '. ~/.cargo/env && cargo run tokio'

# 跨进程共享内存
wsl bash -c '. ~/.cargo/env && cargo run cross-process'
```

## 📊 性能测试

### 运行基准测试

```bash
# 运行所有基准测试
wsl bash -c '. ~/.cargo/env && cargo bench'

# 运行特定基准测试
wsl bash -c '. ~/.cargo/env && cargo bench mutex_contention'
wsl bash -c '. ~/.cargo/env && cargo bench shared_memory'
```

### 运行单元测试

```bash
# 运行所有测试
wsl bash -c '. ~/.cargo/env && cargo test'

# 运行特定测试
wsl bash -c '. ~/.cargo/env && cargo test test_mmap_shared_memory'
wsl bash -c '. ~/.cargo/env && cargo test test_atomic_counter'
```

## 📖 使用示例

### 基本共享内存使用

```rust
use mi7soft::shared_memory::*;

// 创建共享内存
let mut shared_mem = MmapSharedMemory::new(1024)?;

// 写入数据
let data = b"Hello, World!";
shared_mem.write(0, data)?;

// 读取数据
let read_data = shared_mem.read(0, data.len())?;
println!("读取到: {}", String::from_utf8_lossy(&read_data));
```

### 锁机制使用

```rust
use mi7soft::locks::*;
use std::sync::Arc;
use std::thread;

// 使用Mutex
let mutex = Arc::new(StdMutexWrapper::new(0u64));
let mutex_clone = Arc::clone(&mutex);

let handle = thread::spawn(move || {
    let mut guard = mutex_clone.lock().unwrap();
    *guard += 1;
});

handle.join().unwrap();

// 查看性能统计
let stats = mutex.get_stats();
println!("锁统计: {:?}", stats);
```

### 原子操作使用

```rust
use mi7soft::locks::AtomicCounter;
use std::sync::Arc;

let counter = Arc::new(AtomicCounter::new(0));

// 原子递增
let new_value = counter.increment();
println!("新值: {}", new_value);

// 比较并交换
let result = counter.compare_and_swap(1, 10);
match result {
    Ok(old_value) => println!("成功交换，旧值: {}", old_value),
    Err(current) => println!("交换失败，当前值: {}", current),
}
```

## 🏗️ 项目结构

```
src/
├── lib.rs              # 库主入口和错误定义
├── main.rs             # 主程序入口
├── shared_memory.rs    # 共享内存实现
├── locks.rs            # 锁机制实现
├── examples.rs         # 示例代码
└── utils.rs            # 工具函数

benches/
└── shared_memory_bench.rs  # 基准测试

tests/
└── integration_tests.rs    # 集成测试
```

## 🔧 模块说明

### SharedMemory模块
- `MmapSharedMemory`: 基于mmap的共享内存
- `CrossProcessSharedMemory`: 跨进程共享内存
- `SharedMemoryManager`: 共享内存管理器
- `SharedMemoryTrait`: 共享内存统一接口

### Locks模块
- `StdMutexWrapper`: 标准库Mutex包装器
- `StdRwLockWrapper`: 标准库RwLock包装器
- `ParkingMutexWrapper`: Parking Lot Mutex包装器
- `AtomicCounter`: 原子计数器
- `SpinLock`: 自旋锁实现
- `LockStats`: 锁性能统计

### Utils模块
- `SystemInfo`: 系统信息获取
- `PerformanceTester`: 性能测试工具
- `MemoryMonitor`: 内存使用监控
- `DataGenerator`: 测试数据生成
- `Formatter`: 格式化工具

## 📈 性能特点

### 锁性能对比（参考数据）
- **原子操作**: 最快，适合简单计数
- **自旋锁**: 低延迟，适合短时间持锁
- **Parking Lot Mutex**: 高吞吐量，适合高竞争场景
- **标准库Mutex**: 平衡性能，适合一般用途
- **RwLock**: 读多写少场景的最佳选择

### 共享内存性能
- **mmap**: 高性能，适合大块内存操作
- **跨进程共享内存**: 进程间通信的理想选择

## 🔍 监控和调试

项目包含详细的性能监控功能：

- **锁统计**: 锁定次数、等待时间、竞争统计
- **内存监控**: 内存使用变化跟踪
- **系统信息**: CPU、内存、进程数量监控
- **性能测试**: 详细的执行时间统计

## 🤝 贡献指南

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📝 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [shared_memory](https://crates.io/crates/shared_memory) - 跨进程共享内存
- [parking_lot](https://crates.io/crates/parking_lot) - 高性能锁实现
- [tokio](https://crates.io/crates/tokio) - 异步运行时
- [rayon](https://crates.io/crates/rayon) - 并行计算框架
- [criterion](https://crates.io/crates/criterion) - 基准测试框架

## 📞 联系方式

如有问题或建议，请通过以下方式联系：

- 创建 Issue
- 发送 Pull Request
- 邮件联系: [your-email@example.com]

---

⭐ 如果这个项目对您有帮助，请给它一个星标！