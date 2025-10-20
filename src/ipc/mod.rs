//! 跨平台共享内存和锁的封装

use crate::common::{BUFFER_SIZE, Message};
use std::fmt;

// 共享内存中的循环缓冲区结构
#[repr(C)] // 保证内存布局与 C 兼容（跨进程访问需要）
pub struct RingBuffer {
    pub head: usize,      // 读取指针
    pub tail: usize,      // 写入指针
    pub count: usize,     // 当前消息数
    pub buffer: [Option<Message>; BUFFER_SIZE], // 消息缓冲区
}

impl RingBuffer {
    // 初始化空缓冲区
    pub fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            count: 0,
            buffer: std::array::from_fn(|_| None),
        }
    }

    // 写入消息（线程不安全，需外部加锁）
    pub fn push(&mut self, msg: Message) -> Result<(), String> {
        if self.count >= BUFFER_SIZE {
            return Err("缓冲区已满".to_string());
        }
        self.buffer[self.tail] = Some(msg);
        self.tail = (self.tail + 1) % BUFFER_SIZE;
        self.count += 1;
        Ok(())
    }

    // 读取消息（线程不安全，需外部加锁）
    pub fn pop(&mut self) -> Result<Message, String> {
        if self.count == 0 {
            return Err("缓冲区为空".to_string());
        }
        let msg = self.buffer[self.head].take().ok_or("消息不存在")?;
        self.head = (self.head + 1) % BUFFER_SIZE;
        self.count -= 1;
        Ok(msg)
    }
}

// 跨进程锁的 trait
pub trait IpcMutex {
    fn lock(&self) -> Result<(), String>;
    fn unlock(&self) -> Result<(), String>;
}

// 共享内存的 trait
pub trait SharedMemory {
    // 获取共享内存中的缓冲区引用
    fn get_buffer(&self) -> &RingBuffer;
    // 获取共享内存中的缓冲区可变引用（需配合锁使用）
    fn get_buffer_mut(&mut self) -> &mut RingBuffer;
}

// 平台相关实现
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{LinuxIpcMutex, LinuxSharedMemory};

// 统一类型别名（跨平台使用）
#[cfg(target_os = "linux")]
pub type IpcMutexImpl = LinuxIpcMutex;
#[cfg(target_os = "linux")]
pub type SharedMemoryImpl = LinuxSharedMemory;


// 共享内存操作结果
#[derive(Debug)]
pub enum IpcError {
    LockError(String),
    MemoryError(String),
    BufferError(String),
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcError::LockError(e) => write!(f, "锁操作失败: {}", e),
            IpcError::MemoryError(e) => write!(f, "共享内存操作失败: {}", e),
            IpcError::BufferError(e) => write!(f, "缓冲区操作失败: {}", e),
        }
    }
}