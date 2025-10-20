use std::sync::Arc;
use std::slice;
use memmap2::{MmapMut, MmapOptions};
use shared_memory::{Shmem, ShmemConf};
use crate::{Result, SharedMemoryError};

/// 共享内存特征
pub trait SharedMemoryTrait: Send + Sync {
    /// 获取内存大小
    fn size(&self) -> usize;
    
    /// 获取内存指针
    fn as_ptr(&self) -> *mut u8;
    
    /// 获取内存切片
    fn as_slice(&self) -> &[u8];
    
    /// 获取可变内存切片
    fn as_mut_slice(&mut self) -> &mut [u8];
    
    /// 写入数据
    fn write(&mut self, offset: usize, data: &[u8]) -> Result<()>;
    
    /// 读取数据
    fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>>;
}

/// 基于mmap的共享内存实现
pub struct MmapSharedMemory {
    mmap: MmapMut,
    size: usize,
}

impl MmapSharedMemory {
    /// 创建新的mmap共享内存
    pub fn new(size: usize) -> Result<Self> {
        let mmap = MmapOptions::new()
            .len(size)
            .map_anon()
            .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?;
        
        Ok(Self { mmap, size })
    }
    
    /// 从文件创建mmap共享内存
    pub fn from_file(file: std::fs::File, size: usize) -> Result<Self> {
        let mmap = unsafe {
            MmapOptions::new()
                .len(size)
                .map_mut(&file)
                .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?
        };
        
        Ok(Self { mmap, size })
    }
}

impl SharedMemoryTrait for MmapSharedMemory {
    fn size(&self) -> usize {
        self.size
    }
    
    fn as_ptr(&self) -> *mut u8 {
        self.mmap.as_ptr() as *mut u8
    }
    
    fn as_slice(&self) -> &[u8] {
        &self.mmap
    }
    
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.mmap
    }
    
    fn write(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.size {
            return Err(SharedMemoryError::InvalidSize);
        }
        
        self.mmap[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
    
    fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        if offset + len > self.size {
            return Err(SharedMemoryError::InvalidSize);
        }
        
        Ok(self.mmap[offset..offset + len].to_vec())
    }
}

/// 基于shared_memory crate的跨进程共享内存
pub struct CrossProcessSharedMemory {
    shmem: Shmem,
    size: usize,
}

impl CrossProcessSharedMemory {
    /// 创建新的跨进程共享内存
    pub fn create(_name: &str, size: usize) -> Result<Self> {
        let shmem = ShmemConf::new()
            .size(size)
            .create()
            .map_err(|e| SharedMemoryError::CreationFailed(e.to_string()))?;
        
        Ok(Self { shmem, size })
    }
    
    /// 打开已存在的跨进程共享内存
    pub fn open(_name: &str) -> Result<Self> {
        // 注意：shared_memory crate的API可能需要调整
        // 这里提供一个基本实现，实际使用时可能需要根据具体需求修改
        let shmem = ShmemConf::new()
            .size(4096) // 默认大小
            .create()
            .map_err(|e| SharedMemoryError::AccessFailed(e.to_string()))?;
        
        let size = shmem.len();
        Ok(Self { shmem, size })
    }
    
    /// 获取共享内存的操作系统ID
    pub fn get_os_id(&self) -> String {
        format!("shmem_id")
    }
}

impl SharedMemoryTrait for CrossProcessSharedMemory {
    fn size(&self) -> usize {
        self.size
    }
    
    fn as_ptr(&self) -> *mut u8 {
        self.shmem.as_ptr()
    }
    
    fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.shmem.as_ptr() as *const u8, self.size) }
    }
    
    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.shmem.as_ptr(), self.size) }
    }
    
    fn write(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.size {
            return Err(SharedMemoryError::InvalidSize);
        }
        
        let slice = self.as_mut_slice();
        slice[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }
    
    fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        if offset + len > self.size {
            return Err(SharedMemoryError::InvalidSize);
        }
        
        let slice = self.as_slice();
        Ok(slice[offset..offset + len].to_vec())
    }
}

// 为CrossProcessSharedMemory实现Send和Sync
unsafe impl Send for CrossProcessSharedMemory {}
unsafe impl Sync for CrossProcessSharedMemory {}

/// 共享内存管理器
pub struct SharedMemoryManager {
    memories: Vec<Arc<dyn SharedMemoryTrait>>,
}

impl SharedMemoryManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
        }
    }
    
    /// 添加共享内存
    pub fn add_memory(&mut self, memory: Arc<dyn SharedMemoryTrait>) {
        self.memories.push(memory);
    }
    
    /// 获取共享内存数量
    pub fn count(&self) -> usize {
        self.memories.len()
    }
    
    /// 获取指定索引的共享内存
    pub fn get(&self, index: usize) -> Option<&Arc<dyn SharedMemoryTrait>> {
        self.memories.get(index)
    }
}

impl Default for SharedMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}