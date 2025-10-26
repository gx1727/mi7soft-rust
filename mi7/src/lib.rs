pub mod config;
pub mod logging;

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
    pub flag: u8,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl Message {
    const DEFAULT_FLAG: u8 = 0;

    pub fn new(flag: u8, data: String) -> Self {
        Self {
            flag,
            data: data.into_bytes(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn init(data: String) -> Self {
        Self::new(Self::DEFAULT_FLAG, data)
    }
}

/// 队列状态信息
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub capacity: usize,
    pub message_count: usize,
}

pub mod pipe;
pub mod shared_slot;
pub use pipe::{CrossProcessPipe, DefaultCrossProcessPipe, LargeCrossProcessPipe, SmallCrossProcessPipe, PipeStatus, PipeConfig};
pub use shared_slot::{SharedSlotPipe, Slot};
