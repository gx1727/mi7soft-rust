# SharedMemoryMailbox 示例

这个示例演示了如何使用 `SharedMemoryMailbox` 进行进程间通信。

## 功能特性

- 🚀 基于共享内存的高性能进程间通信
- 📦 支持多种大小的消息盒子（1MB, 2MB, 5MB 等）
- 🔒 线程安全的原子操作
- 📊 实时统计信息

## 运行示例

### 1. 启动写入进程

```bash
cargo run --bin shared_box_example writer
```

这将：
- 创建共享内存邮箱
- 写入 4 条不同大小的测试消息
- 显示邮箱统计信息

### 2. 启动读取进程

```bash
cargo run --bin shared_box_example reader
```

这将：
- 连接到现有的共享内存邮箱
- 读取所有可用的消息
- 显示读取的消息内容和统计信息

## 核心概念

### Box 状态
- **Empty**: 空盒子，可以写入
- **Writing**: 正在写入中
- **Full**: 已写入，可以读取
- **Reading**: 正在读取中

### 操作流程

#### 写入流程
1. 获取全局锁
2. 获取空的 box
3. 开始写入（状态: Empty → Writing）
4. 写入数据
5. 完成写入（状态: Writing → Full）

#### 读取流程
1. 获取全局锁
2. 找到满的 box
3. 开始读取（状态: Full → Reading）
4. 读取数据
5. 完成读取（状态: Reading → Empty）

## 示例输出

### 写入进程
```
🚀 启动写入进程...
✅ 共享内存邮箱创建/连接成功
📊 初始统计: MailboxStats { total_count: 10, empty_count: 6, ... }
✅ 消息 1 写入成功，box_id: 8
✅ 消息 2 写入成功，box_id: 9
✅ 消息 3 写入成功，box_id: 4
✅ 消息 4 写入成功，box_id: 2
🎉 写入进程完成！
```

### 读取进程
```
🚀 启动读取进程...
✅ 连接到共享内存邮箱成功
📨 读取消息 1: box_id=2, 内容='Message 4: Large message...'
📨 读取消息 2: box_id=4, 内容='Message 3: Medium sized...'
📨 读取消息 3: box_id=8, 内容='Message 1: Hello from writer!'
📨 读取消息 4: box_id=9, 内容='Message 2: This is a longer...'
🎉 读取进程完成，共读取 4 条消息
```