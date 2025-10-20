//! # 共享内存和锁机制Demo
//! 
//! 这个库提供了多种共享内存实现和锁机制的演示，包括：
//! - 基于mmap的共享内存
//! - 基于shared_memory crate的跨进程共享内存
//! - 多种锁机制：Mutex、RwLock、Atomic操作
//! - 性能测试和基准测试

pub mod shared_memory;
pub mod locks;
pub mod examples;
pub mod utils;
pub mod ipc_queue;

pub use shared_memory::*;
pub use locks::*;

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