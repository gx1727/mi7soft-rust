use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::mem;
use std::ptr;
use shared_memory::{Shmem, ShmemConf};
use serde::{Serialize, Deserialize};
use crate::{Result, SharedMemoryError};

/// 消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// 共享内存中的队列头部信息
#[repr(C)]
struct QueueHeader {
    // 原子操作字段用于无锁操作
    read_pos: AtomicU32,      // 读取位置
    write_pos: AtomicU32,     // 写入位置
    message_count: AtomicU32, // 消息数量
    max_messages: u32,        // 最大消息数
    message_size: u32,        // 每个消息的最大大小
    
    // 用于进程间同步的标志
    writer_lock: AtomicU32,   // 写入锁（简单的自旋锁）
    reader_lock: AtomicU32,   // 读取锁
}

impl QueueHeader {
    fn new(max_messages: u32, message_size: u32) -> Self {
        Self {
            read_pos: AtomicU32::new(0),
            write_pos: AtomicU32::new(0),
            message_count: AtomicU32::new(0),
            max_messages,
            message_size,
            writer_lock: AtomicU32::new(0),
            reader_lock: AtomicU32::new(0),
        }
    }
}

/// 跨进程消息队列
pub struct CrossProcessQueue {
    shmem: Shmem,
    header: *mut QueueHeader,
    data_start: *mut u8,
    max_messages: u32,
    message_size: u32,
}

unsafe impl Send for CrossProcessQueue {}
unsafe impl Sync for CrossProcessQueue {}

impl CrossProcessQueue {
    /// 创建新的跨进程队列（生产者调用）
    pub fn create(name: &str, max_messages: u32, message_size: u32) -> Result<Self> {
        let total_size = mem::size_of::<QueueHeader>() + (max_messages * message_size) as usize;
        
        let shmem = ShmemConf::new()
            .size(total_size)
            .flink(name)
            .create()
            .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?;

        let header = shmem.as_ptr() as *mut QueueHeader;
        let data_start = unsafe { 
            (shmem.as_ptr() as *mut u8).add(mem::size_of::<QueueHeader>()) 
        };

        // 初始化队列头部
        unsafe {
            ptr::write(header, QueueHeader::new(max_messages, message_size));
        }

        Ok(Self {
            shmem,
            header,
            data_start,
            max_messages,
            message_size,
        })
    }

    /// 连接到已存在的队列（消费者调用）
    pub fn connect(name: &str) -> Result<Self> {
        let shmem = ShmemConf::new()
            .flink(name)
            .open()
            .map_err(|e| SharedMemoryError::AccessFailed(e.to_string()))?;

        let header = shmem.as_ptr() as *mut QueueHeader;
        let header_ref = unsafe { &*header };
        
        let max_messages = header_ref.max_messages;
        let message_size = header_ref.message_size;
        
        let data_start = unsafe { 
            (shmem.as_ptr() as *mut u8).add(mem::size_of::<QueueHeader>()) 
        };

        Ok(Self {
            shmem,
            header,
            data_start,
            max_messages,
            message_size,
        })
    }

    /// 发送消息（生产者使用）
    pub fn send(&self, message: &Message) -> Result<()> {
        // 序列化消息
        let serialized = bincode::serialize(message)
            .map_err(|e| SharedMemoryError::SerializationFailed(e.to_string()))?;
        
        if serialized.len() > self.message_size as usize {
            return Err(SharedMemoryError::InvalidSize);
        }

        // 获取写入锁（简单的自旋锁实现）
        self.acquire_writer_lock();

        let header = unsafe { &*self.header };
        
        // 检查队列是否已满
        if header.message_count.load(Ordering::Acquire) >= self.max_messages {
            self.release_writer_lock();
            return Err(SharedMemoryError::QueueFull);
        }

        // 计算写入位置
        let write_pos = header.write_pos.load(Ordering::Acquire);
        let offset = (write_pos * self.message_size) as usize;
        
        // 写入消息长度和数据
        unsafe {
            let msg_ptr = self.data_start.add(offset);
            
            // 写入消息长度（前4字节）
            ptr::write(msg_ptr as *mut u32, serialized.len() as u32);
            
            // 写入消息数据
            ptr::copy_nonoverlapping(
                serialized.as_ptr(),
                msg_ptr.add(4),
                serialized.len()
            );
        }

        // 更新写入位置和消息计数
        let next_write_pos = (write_pos + 1) % self.max_messages;
        header.write_pos.store(next_write_pos, Ordering::Release);
        header.message_count.fetch_add(1, Ordering::Release);

        self.release_writer_lock();
        Ok(())
    }

    /// 接收消息（消费者使用）
    pub fn receive(&self) -> Result<Option<Message>> {
        // 获取读取锁
        self.acquire_reader_lock();

        let header = unsafe { &*self.header };
        
        // 检查是否有消息
        if header.message_count.load(Ordering::Acquire) == 0 {
            self.release_reader_lock();
            return Ok(None);
        }

        // 计算读取位置
        let read_pos = header.read_pos.load(Ordering::Acquire);
        let offset = (read_pos * self.message_size) as usize;
        
        // 读取消息
        let message = unsafe {
            let msg_ptr = self.data_start.add(offset);
            
            // 读取消息长度
            let msg_len = ptr::read(msg_ptr as *const u32) as usize;
            
            if msg_len > self.message_size as usize - 4 {
                self.release_reader_lock();
                return Err(SharedMemoryError::CorruptedData("Invalid message data".to_string()));
            }
            
            // 读取消息数据
            let mut buffer = vec![0u8; msg_len];
            ptr::copy_nonoverlapping(
                msg_ptr.add(4),
                buffer.as_mut_ptr(),
                msg_len
            );
            
            // 反序列化消息
            bincode::deserialize(&buffer)
                .map_err(|e| SharedMemoryError::DeserializationFailed(e.to_string()))?
        };

        // 更新读取位置和消息计数
        let next_read_pos = (read_pos + 1) % self.max_messages;
        header.read_pos.store(next_read_pos, Ordering::Release);
        header.message_count.fetch_sub(1, Ordering::Release);

        self.release_reader_lock();
        Ok(Some(message))
    }

    /// 获取队列状态
    pub fn status(&self) -> QueueStatus {
        let header = unsafe { &*self.header };
        QueueStatus {
            message_count: header.message_count.load(Ordering::Acquire),
            max_messages: header.max_messages,
            is_full: header.message_count.load(Ordering::Acquire) >= header.max_messages,
            is_empty: header.message_count.load(Ordering::Acquire) == 0,
        }
    }

    /// 简单的自旋锁实现 - 获取写入锁
    fn acquire_writer_lock(&self) {
        let header = unsafe { &*self.header };
        while header.writer_lock.compare_exchange_weak(
            0, 1, Ordering::Acquire, Ordering::Relaxed
        ).is_err() {
            std::hint::spin_loop();
        }
    }

    /// 释放写入锁
    fn release_writer_lock(&self) {
        let header = unsafe { &*self.header };
        header.writer_lock.store(0, Ordering::Release);
    }

    /// 获取读取锁
    fn acquire_reader_lock(&self) {
        let header = unsafe { &*self.header };
        while header.reader_lock.compare_exchange_weak(
            0, 1, Ordering::Acquire, Ordering::Relaxed
        ).is_err() {
            std::hint::spin_loop();
        }
    }

    /// 释放读取锁
    fn release_reader_lock(&self) {
        let header = unsafe { &*self.header };
        header.reader_lock.store(0, Ordering::Release);
    }
}

/// 队列状态信息
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub message_count: u32,
    pub max_messages: u32,
    pub is_full: bool,
    pub is_empty: bool,
}

/// 消息队列管理器
pub struct MessageQueueManager {
    queues: std::collections::HashMap<String, Arc<CrossProcessQueue>>,
}

impl MessageQueueManager {
    pub fn new() -> Self {
        Self {
            queues: std::collections::HashMap::new(),
        }
    }

    /// 创建或连接到队列
    pub fn get_or_create_queue(&mut self, name: &str, max_messages: u32, message_size: u32) -> Result<Arc<CrossProcessQueue>> {
        if let Some(queue) = self.queues.get(name) {
            return Ok(Arc::clone(queue));
        }

        // 尝试连接到已存在的队列，如果失败则创建新队列
        let queue = match CrossProcessQueue::connect(name) {
            Ok(q) => q,
            Err(_) => CrossProcessQueue::create(name, max_messages, message_size)?,
        };

        let queue_arc = Arc::new(queue);
        self.queues.insert(name.to_string(), Arc::clone(&queue_arc));
        Ok(queue_arc)
    }
}

impl Default for MessageQueueManager {
    fn default() -> Self {
        Self::new()
    }
}