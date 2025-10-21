//! # 跨进程消息队列库
//! 
//! 这个库提供了基于原生共享内存的跨进程消息队列实现，包括：
//! - 高性能的跨进程消息队列 (使用 memmap2 + shm_open)
//! - 异步和同步的消息发送/接收
//! - 智能等待策略和超时机制
//! - 大数据传输支持

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::mem;
use std::ptr;
use std::ffi::CString;
use std::os::unix::io::{FromRawFd, RawFd};
use memmap2::{MmapMut, MmapOptions};
use serde::{Serialize, Deserialize};
use tokio::time::{sleep, Duration};
use thiserror::Error;

/// 错误类型定义
#[derive(Debug, Error)]
pub enum SharedMemoryError {
    #[error("创建失败: {0}")]
    CreationFailed(String),
    #[error("访问失败: {0}")]
    AccessFailed(String),
    #[error("锁操作失败: {0}")]
    LockFailed(String),
    #[error("无效的大小")]
    InvalidSize,
    #[error("无效的名称")]
    InvalidName,
    #[error("数据损坏: {0}")]
    CorruptedData(String),
    #[error("序列化失败: {0}")]
    SerializationFailed(String),
    #[error("反序列化失败: {0}")]
    DeserializationFailed(String),
    #[error("队列已满")]
    QueueFull,
    #[error("队列为空")]
    QueueEmpty,
    #[error("系统调用失败: {0}")]
    SystemCallFailed(String),
    #[error("内存映射失败: {0}")]
    MmapFailed(String),
}

pub type Result<T> = std::result::Result<T, SharedMemoryError>;

/// 消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl Message {
    pub fn new(data: String, _binary_data: Vec<u8>) -> Self {
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

/// 大数据引用结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeDataReference {
    pub id: String,
    pub size: usize,
    pub storage_type: DataStorageType,
}

/// 数据存储类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataStorageType {
    SharedMemory,
    File(String),
}

/// 大数据管理器
pub struct LargeDataManager {
    base_path: String,
}

impl LargeDataManager {
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }

    pub fn store_data(&self, data: &[u8]) -> Result<LargeDataReference> {
        let id = format!("large_data_{}", rand::random::<u64>());
        let file_path = format!("{}/{}", self.base_path, id);
        
        std::fs::write(&file_path, data)
            .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?;
        
        Ok(LargeDataReference {
            id,
            size: data.len(),
            storage_type: DataStorageType::File(file_path),
        })
    }

    pub fn load_data(&self, reference: &LargeDataReference) -> Result<Vec<u8>> {
        match &reference.storage_type {
            DataStorageType::File(path) => {
                std::fs::read(path)
                    .map_err(|e| SharedMemoryError::AccessFailed(e.to_string()))
            }
            DataStorageType::SharedMemory => {
                Err(SharedMemoryError::AccessFailed("SharedMemory storage not implemented".to_string()))
            }
        }
    }
}

/// 队列头部结构体
#[repr(C)]
#[derive(Debug)]
pub struct QueueHeader {
    pub capacity: AtomicU32,
    pub head: AtomicU32,
    pub tail: AtomicU32,
    pub message_count: AtomicU32,
    pub lock: AtomicU32,
}

impl QueueHeader {
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity: AtomicU32::new(capacity),
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
            message_count: AtomicU32::new(0),
            lock: AtomicU32::new(0),
        }
    }
}

/// 原生共享内存包装器
pub struct NativeSharedMemory {
    fd: RawFd,
    mmap: MmapMut,
    _name: String,
    size: usize,
}

impl NativeSharedMemory {
    /// 创建新的共享内存段
    pub fn create(name: &str, size: usize) -> Result<Self> {
        let c_name = CString::new(name)
            .map_err(|_e| SharedMemoryError::InvalidName)?;
        
        // 使用 shm_open 创建共享内存
        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                0o666,
            )
        };
        
        if fd == -1 {
            return Err(SharedMemoryError::CreationFailed(
                format!("shm_open failed for {}: {}", name, std::io::Error::last_os_error())
            ));
        }
        
        // 设置共享内存大小
        if unsafe { libc::ftruncate(fd, size as i64) } == -1 {
            unsafe { libc::close(fd) };
            return Err(SharedMemoryError::CreationFailed(
                format!("ftruncate failed: {}", std::io::Error::last_os_error())
            ));
        }
        
        // 创建内存映射
        let mmap = unsafe {
            MmapOptions::new()
                .len(size)
                .map_mut(&std::fs::File::from_raw_fd(fd))
                .map_err(|e| SharedMemoryError::MmapFailed(e.to_string()))?
        };
        
        Ok(Self {
            fd,
            mmap,
            _name: name.to_string(),
            size,
        })
    }
    
    /// 连接到现有的共享内存段
    pub fn connect(name: &str) -> Result<Self> {
        let c_name = CString::new(name)
            .map_err(|_e| SharedMemoryError::InvalidName)?;

        // 打开现有的共享内存
        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_RDWR,
                0,
            )
        };
        
        if fd == -1 {
            return Err(SharedMemoryError::AccessFailed(
                format!("shm_open failed for {}: {}", name, std::io::Error::last_os_error())
            ));
        }
        
        // 获取共享内存大小
        let mut stat: libc::stat = unsafe { mem::zeroed() };
        if unsafe { libc::fstat(fd, &mut stat) } == -1 {
            unsafe { libc::close(fd) };
            return Err(SharedMemoryError::AccessFailed(
                format!("fstat failed: {}", std::io::Error::last_os_error())
            ));
        }
        
        let size = stat.st_size as usize;
        
        // 创建内存映射
        let mmap = unsafe {
            MmapOptions::new()
                .len(size)
                .map_mut(&std::fs::File::from_raw_fd(fd))
                .map_err(|e| SharedMemoryError::MmapFailed(e.to_string()))?
        };
        
        Ok(Self {
            fd,
            mmap,
            _name: name.to_string(),
            size,
        })
    }
    
    /// 获取内存映射的可变引用
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }
    
    /// 获取内存映射的不可变引用
    pub fn as_ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }
    
    /// 获取大小
    pub fn len(&self) -> usize {
        self.size
    }
}

impl Drop for NativeSharedMemory {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// 跨进程队列实现
pub struct CrossProcessQueue {
    _shm: NativeSharedMemory,
    header: *mut QueueHeader,
    data_start: *mut u8,
    message_size: usize,
}

unsafe impl Send for CrossProcessQueue {}
unsafe impl Sync for CrossProcessQueue {}

impl CrossProcessQueue {
    const HEADER_SIZE: usize = mem::size_of::<QueueHeader>();
    const DEFAULT_MESSAGE_SIZE: usize = 4096;

    /// 创建新的跨进程队列
    pub fn create(name: &str, capacity: u32) -> Result<Self> {
        let message_size = Self::DEFAULT_MESSAGE_SIZE;
        let total_size = Self::HEADER_SIZE + (capacity as usize * message_size);
        
        let mut shm = NativeSharedMemory::create(name, total_size)?;
        
        // 初始化队列头部
        let header = shm.as_mut_ptr() as *mut QueueHeader;
        unsafe {
            ptr::write(header, QueueHeader::new(capacity));
        }
        
        let data_start = unsafe { shm.as_mut_ptr().add(Self::HEADER_SIZE) };
        
        Ok(Self {
            _shm: shm,
            header,
            data_start,
            message_size,
        })
    }

    /// 连接到现有的跨进程队列
    pub fn connect(name: &str) -> Result<Self> {
        let mut shm = NativeSharedMemory::connect(name)?;
        
        let header = shm.as_mut_ptr() as *mut QueueHeader;
        let data_start = unsafe { shm.as_mut_ptr().add(Self::HEADER_SIZE) };
        
        Ok(Self {
            _shm: shm,
            header,
            data_start,
            message_size: Self::DEFAULT_MESSAGE_SIZE,
        })
    }

    /// 获取队列头部的引用
    fn header(&self) -> &QueueHeader {
        unsafe { &*self.header }
    }

    /// 简单的自旋锁实现
    fn acquire_lock(&self) -> Result<()> {
        let mut attempts = 0;
        while self.header().lock.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
            attempts += 1;
            if attempts > 10000 {
                return Err(SharedMemoryError::LockFailed("获取锁超时".to_string()));
            }
            std::hint::spin_loop();
        }
        Ok(())
    }

    /// 释放锁
    fn release_lock(&self) {
        self.header().lock.store(0, Ordering::Release);
    }

    /// 发送消息
    pub fn send(&self, message: Message) -> Result<()> {
        self.acquire_lock()?;
        
        let result = self.send_internal(message);
        
        self.release_lock();
        result
    }

    fn send_internal(&self, message: Message) -> Result<()> {
        let header = self.header();
        let capacity = header.capacity.load(Ordering::Relaxed);
        let message_count = header.message_count.load(Ordering::Relaxed);
        
        if message_count >= capacity {
            return Err(SharedMemoryError::QueueFull);
        }
        
        let tail = header.tail.load(Ordering::Relaxed);
        let message_data = serde_json::to_vec(&message)
            .map_err(|e| SharedMemoryError::SerializationFailed(e.to_string()))?;
        
        if message_data.len() > self.message_size {
            return Err(SharedMemoryError::InvalidSize);
        }
        
        let slot_ptr = unsafe { self.data_start.add(tail as usize * self.message_size) };
        
        // 写入消息长度
        unsafe {
            ptr::write(slot_ptr as *mut u32, message_data.len() as u32);
            ptr::copy_nonoverlapping(
                message_data.as_ptr(),
                slot_ptr.add(4),
                message_data.len(),
            );
        }
        
        let new_tail = (tail + 1) % capacity;
        header.tail.store(new_tail, Ordering::Relaxed);
        header.message_count.store(message_count + 1, Ordering::Relaxed);
        
        Ok(())
    }

    /// 接收消息
    pub fn receive(&self) -> Result<Message> {
        self.acquire_lock()?;
        
        let result = self.receive_internal();
        
        self.release_lock();
        result
    }

    fn receive_internal(&self) -> Result<Message> {
        let header = self.header();
        let message_count = header.message_count.load(Ordering::Relaxed);
        
        if message_count == 0 {
            return Err(SharedMemoryError::QueueEmpty);
        }
        
        let head = header.head.load(Ordering::Relaxed);
        let capacity = header.capacity.load(Ordering::Relaxed);
        
        let slot_ptr = unsafe { self.data_start.add(head as usize * self.message_size) };
        
        // 读取消息长度
        let message_len = unsafe { ptr::read(slot_ptr as *const u32) } as usize;
        
        if message_len > self.message_size - 4 {
            return Err(SharedMemoryError::CorruptedData("消息长度无效".to_string()));
        }
        
        // 读取消息数据
        let mut message_data = vec![0u8; message_len];
        unsafe {
            ptr::copy_nonoverlapping(
                slot_ptr.add(4),
                message_data.as_mut_ptr(),
                message_len,
            );
        }
        
        let message: Message = serde_json::from_slice(&message_data)
            .map_err(|e| SharedMemoryError::DeserializationFailed(e.to_string()))?;
        
        let new_head = (head + 1) % capacity;
        header.head.store(new_head, Ordering::Relaxed);
        header.message_count.store(message_count - 1, Ordering::Relaxed);
        
        Ok(message)
    }

    /// 获取队列状态
    pub fn status(&self) -> QueueStatus {
        let header = self.header();
        QueueStatus {
            capacity: header.capacity.load(Ordering::Relaxed),
            message_count: header.message_count.load(Ordering::Relaxed),
            head: header.head.load(Ordering::Relaxed),
            tail: header.tail.load(Ordering::Relaxed),
        }
    }

    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.header().message_count.load(Ordering::Relaxed) == 0
    }

    /// 尝试接收消息（非阻塞）
    pub fn try_receive(&self) -> Result<Option<Message>> {
        match self.receive() {
            Ok(message) => Ok(Some(message)),
            Err(SharedMemoryError::QueueEmpty) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 异步发送消息
    pub async fn send_async(&self, message: Message) -> Result<()> {
        loop {
            match self.send(message.clone()) {
                Ok(()) => return Ok(()),
                Err(SharedMemoryError::QueueFull) => {
                    sleep(Duration::from_millis(1)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// 异步接收消息
    pub async fn receive_async(&self) -> Result<Message> {
        loop {
            match self.receive() {
                Ok(message) => return Ok(message),
                Err(SharedMemoryError::QueueEmpty) => {
                    sleep(Duration::from_millis(1)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// 队列状态信息
#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub capacity: u32,
    pub message_count: u32,
    pub head: u32,
    pub tail: u32,
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

    pub fn create_queue(&mut self, name: String, capacity: u32) -> Result<Arc<CrossProcessQueue>> {
        let queue = Arc::new(CrossProcessQueue::create(&name, capacity)?);
        self.queues.insert(name, queue.clone());
        Ok(queue)
    }

    pub fn connect_queue(&mut self, name: String) -> Result<Arc<CrossProcessQueue>> {
        let queue = Arc::new(CrossProcessQueue::connect(&name)?);
        self.queues.insert(name, queue.clone());
        Ok(queue)
    }

    pub fn get_queue(&self, name: &str) -> Option<Arc<CrossProcessQueue>> {
        self.queues.get(name).cloned()
    }

    pub fn remove_queue(&mut self, name: &str) -> Option<Arc<CrossProcessQueue>> {
        self.queues.remove(name)
    }

    pub fn list_queues(&self) -> Vec<String> {
        self.queues.keys().cloned().collect()
    }
}

impl Default for MessageQueueManager {
    fn default() -> Self {
        Self::new()
    }
}

// 添加 rand 依赖的简单实现
mod rand {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static SEED: AtomicU64 = AtomicU64::new(1);
    
    pub fn random<T>() -> T 
    where 
        T: From<u64>
    {
        let current = SEED.load(Ordering::Relaxed);
        let next = current.wrapping_mul(1103515245).wrapping_add(12345);
        SEED.store(next, Ordering::Relaxed);
        T::from(next)
    }
}