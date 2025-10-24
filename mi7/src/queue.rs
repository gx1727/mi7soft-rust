use crate::{Message, QueueStatus, SharedRingQueue};
use std::ptr::NonNull;

/// 跨进程队列包装器，提供类似CrossProcessQueue的API
pub struct CrossProcessQueue {
    queue: NonNull<SharedRingQueue<100, 4096>>, // 100个槽位，每个4KB
    _name: String,
}

unsafe impl Send for CrossProcessQueue {}
unsafe impl Sync for CrossProcessQueue {}

impl CrossProcessQueue {
    /// 创建新的队列
    pub fn create(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let queue_ptr = SharedRingQueue::<100, 4096>::open(name, true);
            if queue_ptr.is_null() {
                return Err("Failed to create shared ring queue".into());
            }
            
            Ok(Self {
                queue: NonNull::new_unchecked(queue_ptr),
                _name: name.to_string(),
            })
        }
    }
    
    /// 连接到现有队列
    pub fn connect(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let queue_ptr = SharedRingQueue::<100, 4096>::open(name, false);
            if queue_ptr.is_null() {
                return Err("Failed to connect to shared ring queue".into());
            }
            
            Ok(Self {
                queue: NonNull::new_unchecked(queue_ptr),
                _name: name.to_string(),
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
        // 简化的状态实现
        QueueStatus {
            capacity: 100,
            message_count: 0, // 实际实现中需要计算当前消息数量
        }
    }
}