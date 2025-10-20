//! # 跨进程消息队列库
//! 
//! 这个库提供了基于共享内存的跨进程消息队列实现，包括：
//! - 高性能的跨进程消息队列
//! - 异步和同步的消息发送/接收
//! - 智能等待策略和超时机制
//! - 大数据传输支持

pub mod ipc_queue;
pub mod producer;
pub mod worker;

/// 错误类型定义
#[derive(Debug)]
pub enum SharedMemoryError {
    CreationFailed(String),
    AccessFailed(String),
    LockFailed(String),
    InvalidSize,
    InvalidName,
    CorruptedData(String),
    SerializationFailed(String),
    DeserializationFailed(String),
    QueueFull,
    QueueEmpty,
}

impl std::fmt::Display for SharedMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SharedMemoryError::CreationFailed(msg) => write!(f, "创建失败: {}", msg),
            SharedMemoryError::AccessFailed(msg) => write!(f, "访问失败: {}", msg),
            SharedMemoryError::LockFailed(msg) => write!(f, "锁操作失败: {}", msg),
            SharedMemoryError::InvalidSize => write!(f, "无效的大小"),
            SharedMemoryError::InvalidName => write!(f, "无效的名称"),
            SharedMemoryError::CorruptedData(msg) => write!(f, "数据损坏: {}", msg),
            SharedMemoryError::SerializationFailed(msg) => write!(f, "序列化失败: {}", msg),
            SharedMemoryError::DeserializationFailed(msg) => write!(f, "反序列化失败: {}", msg),
            SharedMemoryError::QueueFull => write!(f, "队列已满"),
            SharedMemoryError::QueueEmpty => write!(f, "队列为空"),
        }
    }
}

impl std::error::Error for SharedMemoryError {}

pub type Result<T> = std::result::Result<T, SharedMemoryError>;