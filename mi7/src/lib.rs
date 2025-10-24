pub mod config;
pub mod logging;
pub mod queue;
pub mod shared_ring;

// Re-export the main types from shared_ring module
pub use shared_ring::{SharedRingQueue, Slot, SlotState};
// Re-export the queue wrapper and configuration types
pub use queue::{
    CrossProcessQueue, DefaultCrossProcessQueue, LargeCrossProcessQueue, QueueConfig,
    SmallCrossProcessQueue,
};

// Re-export the config types and functions
pub use config::{
    Config, ConfigError, HttpConfig, LoggingConfig, QueueConfig as SystemQueueConfig,
    SharedMemoryConfig, get_config, get_http_bind_address, get_http_config, get_http_port,
    get_log_path, get_logging_config, get_queue_capacity, get_queue_config, get_queue_name,
    get_shared_memory_config, get_shared_memory_name, get_slot_count, get_slot_size, init_config,
};

/// 消息结构体，支持bincode序列化
#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
pub struct Message {
    pub id: u64,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl Message {
    pub fn new(data: String) -> Self {
        Self {
            id: rand::random(),
            data: data.into_bytes(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// 队列状态信息
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub capacity: usize,
    pub message_count: usize,
}

/// 简单的随机数生成器
mod rand {
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEED: AtomicU64 = AtomicU64::new(1);

    pub fn random<T>() -> T
    where
        T: From<u64>,
    {
        let prev = SEED.load(Ordering::Relaxed);
        let next = prev.wrapping_mul(1103515245).wrapping_add(12345);
        SEED.store(next, Ordering::Relaxed);
        T::from(next)
    }
}
