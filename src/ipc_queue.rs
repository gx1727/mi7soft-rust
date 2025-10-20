use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::mem;
use std::ptr;
use shared_memory::{Shmem, ShmemConf};
use serde::{Serialize, Deserialize};
use tokio::time::{sleep, Duration};
use crate::{Result, SharedMemoryError};

/// 消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// 大数据引用消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeDataReference {
    pub message_id: u64,
    pub data_type: DataStorageType,
    pub reference: String,        // 文件路径、共享内存名称等
    pub data_size: u64,          // 数据大小
    pub checksum: u64,           // 数据校验和
    pub timestamp: u64,
}

/// 数据存储类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataStorageType {
    SharedMemory(String),        // 共享内存名称
    TempFile(String),           // 临时文件路径
    MemoryMappedFile(String),   // 内存映射文件
}

/// 大数据管理器
pub struct LargeDataManager {
    temp_dir: std::path::PathBuf,
    shared_memories: std::collections::HashMap<String, Shmem>,
}

impl LargeDataManager {
    pub fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("mi7soft_large_data");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?;
        
        Ok(Self {
            temp_dir,
            shared_memories: std::collections::HashMap::new(),
        })
    }

    /// 存储大数据并返回引用
    pub fn store_large_data(&mut self, data: &[u8]) -> Result<LargeDataReference> {
        let message_id = self.generate_id();
        
        // 根据数据大小选择存储方式
        let (data_type, reference) = if data.len() < 100 * 1024 * 1024 { // 100MB以下用共享内存
            let shmem_name = format!("large_data_{}", message_id);
            let shmem = ShmemConf::new()
                .size(data.len())
                .flink(&shmem_name)
                .create()
                .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?;
            
            // 复制数据到共享内存
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    shmem.as_ptr(),
                    data.len()
                );
            }
            
            self.shared_memories.insert(shmem_name.clone(), shmem);
            (DataStorageType::SharedMemory(shmem_name.clone()), shmem_name)
        } else { // 大数据用临时文件
            let file_path = self.temp_dir.join(format!("large_data_{}.bin", message_id));
            std::fs::write(&file_path, data)
                .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?;
            
            let path_str = file_path.to_string_lossy().to_string();
            (DataStorageType::TempFile(path_str.clone()), path_str)
        };
        
        Ok(LargeDataReference {
            message_id,
            data_type,
            reference,
            data_size: data.len() as u64,
            checksum: self.calculate_checksum(data),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// 根据引用读取大数据
    pub fn load_large_data(&mut self, reference: &LargeDataReference) -> Result<Vec<u8>> {
        match &reference.data_type {
            DataStorageType::SharedMemory(name) => {
                if let Some(shmem) = self.shared_memories.get(name) {
                    let data = unsafe {
                        std::slice::from_raw_parts(
                            shmem.as_ptr(),
                            reference.data_size as usize
                        ).to_vec()
                    };
                    
                    // 验证校验和
                    if self.calculate_checksum(&data) != reference.checksum {
                        return Err(SharedMemoryError::CorruptedData("Checksum mismatch".to_string()));
                    }
                    
                    Ok(data)
                } else {
                    // 尝试连接到已存在的共享内存
                    let shmem = ShmemConf::new()
                        .flink(name)
                        .open()
                        .map_err(|e| SharedMemoryError::AccessFailed(e.to_string()))?;
                    
                    let data = unsafe {
                        std::slice::from_raw_parts(
                            shmem.as_ptr(),
                            reference.data_size as usize
                        ).to_vec()
                    };
                    
                    self.shared_memories.insert(name.clone(), shmem);
                    Ok(data)
                }
            }
            DataStorageType::TempFile(path) => {
                let data = std::fs::read(path)
                    .map_err(|e| SharedMemoryError::AccessFailed(e.to_string()))?;
                
                // 验证校验和
                if self.calculate_checksum(&data) != reference.checksum {
                    return Err(SharedMemoryError::CorruptedData("Checksum mismatch".to_string()));
                }
                
                Ok(data)
            }
            DataStorageType::MemoryMappedFile(path) => {
                // 使用 memmap2 实现内存映射文件读取
                let file = std::fs::File::open(path)
                    .map_err(|e| SharedMemoryError::AccessFailed(e.to_string()))?;
                
                let mmap = unsafe { memmap2::Mmap::map(&file) }
                    .map_err(|e| SharedMemoryError::AccessFailed(e.to_string()))?;
                
                let data = mmap.to_vec();
                
                // 验证校验和
                if self.calculate_checksum(&data) != reference.checksum {
                    return Err(SharedMemoryError::CorruptedData("Checksum mismatch".to_string()));
                }
                
                Ok(data)
            }
        }
    }

    fn generate_id(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    fn calculate_checksum(&self, data: &[u8]) -> u64 {
        // 简单的校验和实现（实际应用中应该使用更强的算法如CRC32）
        data.iter().map(|&b| b as u64).sum()
    }
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
        let header = unsafe { &*self.header };
        
        // 先快速检查是否有消息，避免无意义的锁竞争
        if header.message_count.load(Ordering::Acquire) == 0 {
            return Ok(None);
        }

        // 获取读取锁
        self.acquire_reader_lock();
        
        // 再次检查是否有消息（双重检查，防止竞争条件）
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

    /// 快速检查队列是否为空（无锁操作）
    pub fn is_empty(&self) -> bool {
        let header = unsafe { &*self.header };
        header.message_count.load(Ordering::Acquire) == 0
    }

    /// 非阻塞尝试接收消息
    /// 如果队列为空，立即返回 None 而不等待
    pub fn try_receive(&self) -> Result<Option<Message>> {
        // 快速检查，避免锁竞争
        if self.is_empty() {
            return Ok(None);
        }
        
        // 有消息时才调用正常的 receive
        self.receive()
    }

    /// 简单的自旋锁实现 - 获取写入锁
    fn acquire_writer_lock(&self) {
        let mut spin_count = 0;
        let header = unsafe { &*self.header };
        while header.writer_lock.compare_exchange_weak(
            0, 1, Ordering::Acquire, Ordering::Relaxed
        ).is_err() {
            spin_count += 1;
            
            if spin_count < 100 {
                std::hint::spin_loop(); // 先自旋
            } else {
                std::thread::yield_now(); // 然后让出CPU
            }
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

    // ==================== 异步方法 ====================

    /// 异步发送消息（生产者使用）
    /// 如果队列满了，会等待一段时间后重试
    pub async fn send_async(&self, message: &Message) -> Result<()> {
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 10;
        
        loop {
            match self.send(message) {
                Ok(()) => return Ok(()),
                Err(SharedMemoryError::QueueFull) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        return Err(SharedMemoryError::QueueFull);
                    }
                    
                    // 指数退避等待
                    let wait_time = Duration::from_millis(10 * (1 << retry_count.min(6)));
                    sleep(wait_time).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 异步接收消息（消费者使用）
    /// 如果队列为空，会等待一段时间后重试
    pub async fn receive_async(&self) -> Result<Option<Message>> {
        self.receive_async_with_timeout(Duration::from_secs(30)).await
    }

    /// 异步接收消息，带超时
    pub async fn receive_async_with_timeout(&self, timeout: Duration) -> Result<Option<Message>> {
        let start_time = std::time::Instant::now();
        let mut consecutive_empty = 0;
        
        loop {
            // 检查超时
            if start_time.elapsed() >= timeout {
                return Ok(None);
            }
            
            // 尝试接收消息
            match self.try_receive()? {
                Some(message) => return Ok(Some(message)),
                None => {
                    consecutive_empty += 1;
                    
                    // 智能等待策略：开始时等待时间短，逐渐增加
                    let wait_time = if consecutive_empty < 10 {
                        Duration::from_millis(1)  // 前10次快速轮询
                    } else if consecutive_empty < 50 {
                        Duration::from_millis(10) // 接下来40次中等等待
                    } else {
                        Duration::from_millis(100) // 之后长等待
                    };
                    
                    sleep(wait_time).await;
                }
            }
        }
    }

    /// 异步等待直到有消息可用
    pub async fn wait_for_message(&self) -> Result<Message> {
        loop {
            if let Some(message) = self.try_receive()? {
                return Ok(message);
            }
            
            // 短暂等待后重试
            sleep(Duration::from_millis(1)).await;
        }
    }

    /// 异步批量接收消息
    pub async fn receive_batch_async(&self, max_count: usize) -> Result<Vec<Message>> {
        let mut messages = Vec::with_capacity(max_count);
        
        // 首先尝试快速收集已有的消息
        while messages.len() < max_count {
            match self.try_receive()? {
                Some(message) => messages.push(message),
                None => break,
            }
        }
        
        // 如果没有收集到任何消息，等待至少一个消息
        if messages.is_empty() {
            if let Some(message) = self.receive_async_with_timeout(Duration::from_secs(5)).await? {
                messages.push(message);
                
                // 尝试收集更多消息（非阻塞）
                while messages.len() < max_count {
                    match self.try_receive()? {
                        Some(message) => messages.push(message),
                        None => break,
                    }
                }
            }
        }
        
        Ok(messages)
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