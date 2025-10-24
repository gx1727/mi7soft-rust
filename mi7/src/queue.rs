use crate::{Message, QueueStatus, SharedRingQueue};
use std::ptr::NonNull;

/// 队列配置结构体
#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    /// 队列槽位数量
    pub capacity: usize,
    /// 每个槽位的大小（字节）
    pub slot_size: usize,
}

impl QueueConfig {
    /// 创建新的队列配置
    pub fn new(capacity: usize, slot_size: usize) -> Self {
        Self {
            capacity,
            slot_size,
        }
    }

    /// 默认配置：100个槽位，每个4KB
    pub fn default() -> Self {
        Self {
            capacity: 100,
            slot_size: 4096,
        }
    }

    /// 小型队列配置：10个槽位，每个1KB
    pub fn small() -> Self {
        Self {
            capacity: 10,
            slot_size: 1024,
        }
    }

    /// 大型队列配置：1000个槽位，每个8KB
    pub fn large() -> Self {
        Self {
            capacity: 1000,
            slot_size: 8192,
        }
    }
}

/// 跨进程队列包装器，提供类似CrossProcessQueue的API
/// 支持配置化的队列大小和槽位大小
pub struct CrossProcessQueue<const CAPACITY: usize, const SLOT_SIZE: usize> {
    queue: NonNull<SharedRingQueue<CAPACITY, SLOT_SIZE>>,
    _name: String,
    config: QueueConfig,
}

unsafe impl<const CAPACITY: usize, const SLOT_SIZE: usize> Send
    for CrossProcessQueue<CAPACITY, SLOT_SIZE>
{
}
unsafe impl<const CAPACITY: usize, const SLOT_SIZE: usize> Sync
    for CrossProcessQueue<CAPACITY, SLOT_SIZE>
{
}

impl<const CAPACITY: usize, const SLOT_SIZE: usize> CrossProcessQueue<CAPACITY, SLOT_SIZE> {
    /// 创建新的队列
    pub fn create(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let queue_ptr = SharedRingQueue::<CAPACITY, SLOT_SIZE>::open(name, true);
            if queue_ptr.is_null() {
                return Err("Failed to create shared ring queue".into());
            }

            Ok(Self {
                queue: NonNull::new_unchecked(queue_ptr),
                _name: name.to_string(),
                config: QueueConfig::new(CAPACITY, SLOT_SIZE),
            })
        }
    }

    /// 使用配置创建新的队列
    pub fn create_with_config(
        name: &str,
        _config: QueueConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 注意：配置参数在编译时确定，这里主要用于验证
        if _config.capacity != CAPACITY || _config.slot_size != SLOT_SIZE {
            return Err(format!(
                "配置不匹配：期望 capacity={}, slot_size={}，实际 capacity={}, slot_size={}",
                CAPACITY, SLOT_SIZE, _config.capacity, _config.slot_size
            )
            .into());
        }
        Self::create(name)
    }

    /// 连接到现有队列
    pub fn connect(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let queue_ptr = SharedRingQueue::<CAPACITY, SLOT_SIZE>::open(name, false);
            if queue_ptr.is_null() {
                return Err("Failed to connect to shared ring queue".into());
            }

            Ok(Self {
                queue: NonNull::new_unchecked(queue_ptr),
                _name: name.to_string(),
                config: QueueConfig::new(CAPACITY, SLOT_SIZE),
            })
        }
    }

    /// 发送消息
    pub fn send(&self, message: Message) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let queue = self.queue.as_ptr();
            match (*queue).push(&message) {
                Ok(_request_id) => Ok(()),
                Err(err) => Err(err.into()),
            }
        }
    }

    /// 接收消息（阻塞）
    pub fn receive(&self) -> Result<Message, Box<dyn std::error::Error>> {
        unsafe {
            let queue = self.queue.as_ptr();
            if let Some((_, message)) = (*queue).pop::<Message>() {
                Ok(message)
            } else {
                Err("Failed to receive message".into())
            }
        }
    }

    /// 尝试接收消息（非阻塞）
    pub fn try_receive(&self) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        unsafe {
            let queue = self.queue.as_ptr();
            if let Some((_, message)) = (*queue).pop::<Message>() {
                Ok(Some(message))
            } else {
                Ok(None)
            }
        }
    }

    /// 获取队列状态
    pub fn status(&self) -> QueueStatus {
        QueueStatus {
            capacity: CAPACITY,
            message_count: 0, // 实际实现中需要计算当前消息数量
        }
    }

    /// 获取队列配置
    pub fn config(&self) -> QueueConfig {
        self.config
    }

    /// 获取队列容量
    pub fn capacity(&self) -> usize {
        CAPACITY
    }

    /// 获取槽位大小
    pub fn slot_size(&self) -> usize {
        SLOT_SIZE
    }
}

// 为了向后兼容，提供默认配置的类型别名
pub type DefaultCrossProcessQueue = CrossProcessQueue<100, 4096>;

// 提供一些常用配置的类型别名
pub type SmallCrossProcessQueue = CrossProcessQueue<10, 1024>;
pub type LargeCrossProcessQueue = CrossProcessQueue<1000, 8192>;

// 为默认配置提供便捷方法
impl DefaultCrossProcessQueue {
    /// 使用默认配置创建队列
    pub fn create_default(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::create(name)
    }

    /// 使用默认配置连接队列
    pub fn connect_default(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect(name)
    }
}
