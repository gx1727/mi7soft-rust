use anyhow::{Result, anyhow};
use std::alloc::{Layout, alloc, dealloc};
use std::collections::HashMap;
use std::mem;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Box 状态枚举
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoxState {
    Empty = 0,   // 空
    Writing = 1, // 写入中
    Full = 2,    // 满
    Reading = 3, // 读取中
}

impl From<u8> for BoxState {
    fn from(value: u8) -> Self {
        match value {
            0 => BoxState::Empty,
            1 => BoxState::Writing,
            2 => BoxState::Full,
            3 => BoxState::Reading,
            _ => BoxState::Empty,
        }
    }
}

/// Box 大小类型 (MB)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoxSize {
    Size1M = 1,
    Size2M = 2,
    Size3M = 3,
    Size4M = 4,
    Size5M = 5,
    Size6M = 6,
    Size7M = 7,
    Size8M = 8,
    Size9M = 9,
    Size10M = 10,
    Size20M = 20,
    Size50M = 50,
    Size100M = 100,
}

impl BoxSize {
    pub fn bytes(&self) -> usize {
        (*self as usize) * 1024 * 1024
    }

    pub fn all_sizes() -> Vec<BoxSize> {
        vec![
            BoxSize::Size1M,
            BoxSize::Size2M,
            BoxSize::Size3M,
            BoxSize::Size4M,
            BoxSize::Size5M,
            BoxSize::Size6M,
            BoxSize::Size7M,
            BoxSize::Size8M,
            BoxSize::Size9M,
            BoxSize::Size10M,
            BoxSize::Size20M,
            BoxSize::Size50M,
            BoxSize::Size100M,
        ]
    }

    /// 获取每种大小的 box 数量配置
    pub fn get_box_count(&self) -> usize {
        match self {
            BoxSize::Size1M => 10,
            BoxSize::Size2M => 5,
            BoxSize::Size3M => 0, // 不使用
            BoxSize::Size4M => 0, // 不使用
            BoxSize::Size5M => 2,
            BoxSize::Size6M => 0, // 不使用
            BoxSize::Size7M => 0, // 不使用
            BoxSize::Size8M => 0, // 不使用
            BoxSize::Size9M => 0, // 不使用
            BoxSize::Size10M => 1,
            BoxSize::Size20M => 0,  // 不使用
            BoxSize::Size50M => 0,  // 不使用
            BoxSize::Size100M => 0, // 不使用
        }
    }
}

/// Box 元数据
#[repr(C)]
pub struct BoxMetadata {
    pub id: AtomicU32,          // Box ID
    pub state: AtomicU8,        // Box 状态
    pub size: AtomicU32,        // Box 大小 (MB)
    pub data_length: AtomicU32, // 实际数据长度
    pub data_ptr: AtomicU32,    // 数据指针偏移量
}

impl BoxMetadata {
    pub fn new(id: u32, size: BoxSize, data_offset: u32) -> Self {
        Self {
            id: AtomicU32::new(id),
            state: AtomicU8::new(BoxState::Empty as u8),
            size: AtomicU32::new(size as u32),
            data_length: AtomicU32::new(0),
            data_ptr: AtomicU32::new(data_offset),
        }
    }

    pub fn get_state(&self) -> BoxState {
        BoxState::from(self.state.load(Ordering::Acquire))
    }

    pub fn set_state(&self, state: BoxState) {
        self.state.store(state as u8, Ordering::Release);
    }

    pub fn get_id(&self) -> u32 {
        self.id.load(Ordering::Relaxed)
    }

    pub fn get_size(&self) -> BoxSize {
        match self.size.load(Ordering::Relaxed) {
            1 => BoxSize::Size1M,
            2 => BoxSize::Size2M,
            3 => BoxSize::Size3M,
            4 => BoxSize::Size4M,
            5 => BoxSize::Size5M,
            6 => BoxSize::Size6M,
            7 => BoxSize::Size7M,
            8 => BoxSize::Size8M,
            9 => BoxSize::Size9M,
            10 => BoxSize::Size10M,
            _ => BoxSize::Size1M,
        }
    }

    pub fn get_data_length(&self) -> u32 {
        self.data_length.load(Ordering::Acquire)
    }

    pub fn set_data_length(&self, length: u32) {
        self.data_length.store(length, Ordering::Release);
    }

    pub fn get_data_offset(&self) -> u32 {
        self.data_ptr.load(Ordering::Relaxed)
    }
}

/// 共享内存寄存箱头部信息
#[repr(C)]
pub struct MailboxHeader {
    pub magic: AtomicU32,       // 魔数，用于验证
    pub version: AtomicU32,     // 版本号
    pub total_boxes: AtomicU32, // 总 box 数量
    pub lock: AtomicU32,        // 全局锁 (0=未锁定, 1=锁定)
    pub next_box_id: AtomicU32, // 下一个 box ID
}

impl MailboxHeader {
    const MAGIC: u32 = 0x4D41494C; // "MAIL"
    const VERSION: u32 = 1;

    pub fn new(total_boxes: u32) -> Self {
        Self {
            magic: AtomicU32::new(Self::MAGIC),
            version: AtomicU32::new(Self::VERSION),
            total_boxes: AtomicU32::new(total_boxes),
            lock: AtomicU32::new(0),
            next_box_id: AtomicU32::new(1),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic.load(Ordering::Relaxed) == Self::MAGIC
            && self.version.load(Ordering::Relaxed) == Self::VERSION
    }

    /// 尝试获取全局锁
    pub fn try_lock(&self) -> bool {
        self.lock
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    /// 释放全局锁
    pub fn unlock(&self) {
        self.lock.store(0, Ordering::Release);
    }

    /// 获取下一个 box ID
    pub fn next_id(&self) -> u32 {
        self.next_box_id.fetch_add(1, Ordering::AcqRel)
    }
}

/// 共享内存寄存箱
pub struct SharedMailbox {
    memory: NonNull<u8>,
    size: usize,
    header: *mut MailboxHeader,
    boxes: Vec<*mut BoxMetadata>,
    box_index: HashMap<BoxSize, Vec<usize>>, // 按大小分类的 box 索引
}

unsafe impl Send for SharedMailbox {}
unsafe impl Sync for SharedMailbox {}

impl SharedMailbox {
    /// 计算所需的总内存大小
    pub fn calculate_memory_size() -> usize {
        let header_size = mem::size_of::<MailboxHeader>();
        let mut total_size = header_size;

        // 计算所有 box 的元数据大小
        let mut total_boxes = 0;
        for size in BoxSize::all_sizes() {
            total_boxes += size.get_box_count();
        }

        let metadata_size = total_boxes * mem::size_of::<BoxMetadata>();
        total_size += metadata_size;

        // 计算所有 box 的数据大小
        for size in BoxSize::all_sizes() {
            total_size += size.get_box_count() * size.bytes();
        }

        // 对齐到页边界
        let page_size = 4096;
        (total_size + page_size - 1) & !(page_size - 1)
    }

    /// 创建新的共享内存寄存箱
    pub fn new() -> Result<Self> {
        let total_size = Self::calculate_memory_size();

        // 分配内存
        let layout = Layout::from_size_align(total_size, 4096)
            .map_err(|e| anyhow!("Failed to create layout: {}", e))?;

        let memory = unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                return Err(anyhow!("Failed to allocate memory"));
            }
            // 初始化内存为 0
            std::ptr::write_bytes(ptr, 0, total_size);
            NonNull::new_unchecked(ptr)
        };

        let mut mailbox = Self {
            memory,
            size: total_size,
            header: memory.as_ptr() as *mut MailboxHeader,
            boxes: Vec::new(),
            box_index: HashMap::new(),
        };

        mailbox.initialize()?;
        Ok(mailbox)
    }

    /// 初始化寄存箱
    fn initialize(&mut self) -> Result<()> {
        // 计算总 box 数量
        let mut total_boxes = 0;
        for size in BoxSize::all_sizes() {
            total_boxes += size.get_box_count();
        }

        // 初始化头部
        unsafe {
            std::ptr::write(self.header, MailboxHeader::new(total_boxes as u32));
        }

        // 计算各部分的偏移量
        let header_size = mem::size_of::<MailboxHeader>();
        let metadata_start = header_size;
        let metadata_size = total_boxes * mem::size_of::<BoxMetadata>();
        let data_start = metadata_start + metadata_size;

        let mut box_id = 1u32;
        let mut metadata_offset = metadata_start;
        let mut data_offset = data_start;

        // 为每种大小的 box 创建元数据和索引
        for size in BoxSize::all_sizes() {
            let count = size.get_box_count();
            let mut size_indices = Vec::new();

            for _ in 0..count {
                // 创建 box 元数据
                let metadata_ptr = unsafe {
                    let ptr = self.memory.as_ptr().add(metadata_offset) as *mut BoxMetadata;
                    std::ptr::write(ptr, BoxMetadata::new(box_id, size, data_offset as u32));
                    ptr
                };

                self.boxes.push(metadata_ptr);
                size_indices.push(self.boxes.len() - 1);

                box_id += 1;
                metadata_offset += mem::size_of::<BoxMetadata>();
                data_offset += size.bytes();
            }

            self.box_index.insert(size, size_indices);
        }

        Ok(())
    }

    /// 获取全局锁
    pub fn lock(&self) -> Result<MailboxLock> {
        let header = unsafe { &*self.header };

        // 改进的等待策略：先自旋，然后休眠
        let mut attempts = 0;
        while !header.try_lock() {
            attempts += 1;
            if attempts > 100000 {
                return Err(anyhow!("Failed to acquire lock after 100000 attempts"));
            }

            if attempts < 1000 {
                // 前1000次尝试使用 yield
                std::thread::yield_now();
            } else {
                // 之后使用短暂休眠
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }

        Ok(MailboxLock { header })
    }

    /// 获取指定大小的空 box
    pub fn get_empty_box(&self, size: BoxSize) -> Result<u32> {
        let indices = self
            .box_index
            .get(&size)
            .ok_or_else(|| anyhow!("Invalid box size: {:?}", size))?;

        for &index in indices {
            let metadata = unsafe { &*self.boxes[index] };
            if metadata.get_state() == BoxState::Empty {
                metadata.set_state(BoxState::Writing);
                return Ok(metadata.get_id());
            }
        }

        Err(anyhow!("No empty box available for size: {:?}", size))
    }

    /// 根据 ID 查找 box
    pub fn find_box_by_id(&self, box_id: u32) -> Result<&BoxMetadata> {
        for &metadata_ptr in &self.boxes {
            let metadata = unsafe { &*metadata_ptr };
            if metadata.get_id() == box_id {
                return Ok(metadata);
            }
        }
        Err(anyhow!("Box not found: {}", box_id))
    }

    /// 写入数据到 box
    pub fn write_data(&self, box_id: u32, data: &[u8]) -> Result<()> {
        let metadata = self.find_box_by_id(box_id)?;

        if metadata.get_state() != BoxState::Writing {
            return Err(anyhow!("Box {} is not in writing state", box_id));
        }

        let max_size = metadata.get_size().bytes();
        if data.len() > max_size {
            return Err(anyhow!("Data too large: {} > {}", data.len(), max_size));
        }

        // 写入数据
        let data_offset = metadata.get_data_offset() as usize;
        unsafe {
            let data_ptr = self.memory.as_ptr().add(data_offset);
            std::ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
        }

        metadata.set_data_length(data.len() as u32);
        metadata.set_state(BoxState::Full);

        Ok(())
    }

    /// 读取 box 中的数据
    pub fn read_data(&self, box_id: u32) -> Result<Vec<u8>> {
        let metadata = self.find_box_by_id(box_id)?;

        if metadata.get_state() != BoxState::Reading {
            return Err(anyhow!("Box {} is not in reading state", box_id));
        }

        let data_length = metadata.get_data_length() as usize;
        let data_offset = metadata.get_data_offset() as usize;

        let mut data = vec![0u8; data_length];
        unsafe {
            let data_ptr = self.memory.as_ptr().add(data_offset);
            std::ptr::copy_nonoverlapping(data_ptr, data.as_mut_ptr(), data_length);
        }

        Ok(data)
    }

    /// 设置 box 状态为读取中
    pub fn start_reading(&self, box_id: u32) -> Result<()> {
        let metadata = self.find_box_by_id(box_id)?;

        if metadata.get_state() != BoxState::Full {
            return Err(anyhow!("Box {} is not full", box_id));
        }

        metadata.set_state(BoxState::Reading);
        Ok(())
    }

    /// 完成读取，将 box 状态设置为空
    pub fn finish_reading(&self, box_id: u32) -> Result<()> {
        let metadata = self.find_box_by_id(box_id)?;

        if metadata.get_state() != BoxState::Reading {
            return Err(anyhow!("Box {} is not in reading state", box_id));
        }

        metadata.set_data_length(0);
        metadata.set_state(BoxState::Empty);
        Ok(())
    }

    /// 获取寄存箱统计信息
    pub fn get_stats(&self) -> MailboxStats {
        let mut stats = MailboxStats::default();

        for &metadata_ptr in &self.boxes {
            let metadata = unsafe { &*metadata_ptr };
            let state = metadata.get_state();
            let size = metadata.get_size();

            match state {
                BoxState::Empty => stats.empty_count += 1,
                BoxState::Writing => stats.writing_count += 1,
                BoxState::Full => stats.full_count += 1,
                BoxState::Reading => stats.reading_count += 1,
            }

            *stats.size_counts.entry(size).or_insert(0) += 1;
        }

        stats.total_count = self.boxes.len();
        stats
    }
}

impl Drop for SharedMailbox {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, 4096);
            dealloc(self.memory.as_ptr(), layout);
        }
    }
}

/// 寄存箱锁
pub struct MailboxLock<'a> {
    header: &'a MailboxHeader,
}

impl<'a> Drop for MailboxLock<'a> {
    fn drop(&mut self) {
        self.header.unlock();
    }
}

/// 寄存箱统计信息
#[derive(Debug, Default)]
pub struct MailboxStats {
    pub total_count: usize,
    pub empty_count: usize,
    pub writing_count: usize,
    pub full_count: usize,
    pub reading_count: usize,
    pub size_counts: HashMap<BoxSize, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailbox_creation() {
        let mailbox = SharedMailbox::new().unwrap();
        let stats = mailbox.get_stats();

        // 验证所有 box 都是空的
        assert_eq!(stats.empty_count, stats.total_count);
        assert_eq!(stats.writing_count, 0);
        assert_eq!(stats.full_count, 0);
        assert_eq!(stats.reading_count, 0);
    }

    #[test]
    fn test_box_allocation() {
        let mailbox = SharedMailbox::new().unwrap();
        let _lock = mailbox.lock().unwrap();

        // 获取一个 1M 的 box
        let box_id = mailbox.get_empty_box(BoxSize::Size1M).unwrap();
        assert!(box_id > 0);

        // 验证 box 状态
        let metadata = mailbox.find_box_by_id(box_id).unwrap();
        assert_eq!(metadata.get_state(), BoxState::Writing);
    }

    #[test]
    fn test_data_write_read() {
        let mailbox = SharedMailbox::new().unwrap();
        let _lock = mailbox.lock().unwrap();

        // 获取 box 并写入数据
        let box_id = mailbox.get_empty_box(BoxSize::Size1M).unwrap();
        let test_data = b"Hello, Mailbox!";
        mailbox.write_data(box_id, test_data).unwrap();

        // 开始读取
        mailbox.start_reading(box_id).unwrap();
        let read_data = mailbox.read_data(box_id).unwrap();

        assert_eq!(read_data, test_data);

        // 完成读取
        mailbox.finish_reading(box_id).unwrap();

        // 验证 box 状态回到空
        let metadata = mailbox.find_box_by_id(box_id).unwrap();
        assert_eq!(metadata.get_state(), BoxState::Empty);
    }
}
