use libc::{
    EOWNERDEAD, MAP_FAILED, MAP_SHARED, O_CREAT, O_RDWR, PROT_READ, PROT_WRITE,
    PTHREAD_MUTEX_ROBUST, PTHREAD_PROCESS_SHARED, close, ftruncate, mmap, pthread_mutex_consistent,
    pthread_mutex_init, pthread_mutex_lock, pthread_mutex_t, pthread_mutex_unlock,
    pthread_mutexattr_init, pthread_mutexattr_setpshared, pthread_mutexattr_setrobust,
    pthread_mutexattr_t,
};

use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::{ffi::CString, mem, ptr};

/// Tokio IPC 错误类型
#[derive(Debug)]
pub enum TokioIPCError {
    ShmOpenFailed(i32),
    MmapFailed,
    MutexLockFailed,
    QueueFull,
    SerializationFailed,
    ChecksumMismatch,
    SlotNotReady,
}

impl std::fmt::Display for TokioIPCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokioIPCError::ShmOpenFailed(code) => {
                write!(f, "Shared memory open failed with code: {}", code)
            }
            TokioIPCError::MmapFailed => write!(f, "Memory mapping failed"),
            TokioIPCError::MutexLockFailed => write!(f, "Mutex lock failed"),
            TokioIPCError::QueueFull => write!(f, "Queue is full"),
            TokioIPCError::SerializationFailed => write!(f, "Serialization failed"),
            TokioIPCError::ChecksumMismatch => write!(f, "Checksum mismatch"),
            TokioIPCError::SlotNotReady => write!(f, "Slot is not ready"),
        }
    }
}

impl std::error::Error for TokioIPCError {}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SlotState {
    EMPTY = 0,
    WRITING = 1,
    INPROGRESS = 2,
    READING = 3,
    READY = 4,
}

#[repr(C)]
pub struct Slot<const SLOT_SIZE: usize> {
    pub state: AtomicU32, // 简化的原子状态
    pub request_id: u64,  // 请求ID
    pub data_size: u32,   // 实际数据大小
    pub checksum: u64,    // 数据校验和
    pub data: [u8; SLOT_SIZE],
}

#[repr(C)]
pub struct SharedSlotPipe<const N: usize, const SLOT_SIZE: usize> {
    pub write_mutex: pthread_mutex_t, // 保护写操作
    pub read_mutex: pthread_mutex_t,  // 保护读操作
    pub write_pointer: usize,         // 可写的索引
    pub read_pointer: usize,          // 可读的索引
    pub slots: [Slot<SLOT_SIZE>; N],
    pub seq: AtomicU64,          // request_id 生成器
    pub begin: AtomicBool,       // "有数据"信号（原子变量，线程安全）
    pub shared_value: AtomicU32, // AsyncFutex 使用
}

unsafe impl<const N: usize, const SLOT_SIZE: usize> Send for SharedSlotPipe<N, SLOT_SIZE> {}

unsafe impl<const N: usize, const SLOT_SIZE: usize> Sync for SharedSlotPipe<N, SLOT_SIZE> {}

impl<const N: usize, const SLOT_SIZE: usize> SharedSlotPipe<N, SLOT_SIZE> {
    /// 打开或创建共享内存
    pub unsafe fn open(name: &str, create: bool) -> Result<*mut Self> {
        let cname = if name.starts_with('/') {
            CString::new(name)
        } else {
            CString::new(format!("/{}", name))
        };

        let cname = match cname {
            Ok(name) => name,
            Err(_) => {
                return Err(anyhow::anyhow!("Failed to create CString from name"));
            }
        };

        let flags = if create { O_CREAT | O_RDWR } else { O_RDWR };
        let fd = unsafe { libc::shm_open(cname.as_ptr(), flags, 0o666) };
        if fd == -1 {
            return Err(anyhow::anyhow!("shm_open failed with errno: {}", unsafe {
                *libc::__errno_location()
            }));
        }

        if create {
            if unsafe { ftruncate(fd, mem::size_of::<Self>() as i64) } == -1 {
                unsafe { close(fd) };
                return Err(anyhow::anyhow!("ftruncate failed with errno: {}", unsafe {
                    *libc::__errno_location()
                }));
            }
        }

        let size = mem::size_of::<Self>();

        let addr = unsafe {
            mmap(
                ptr::null_mut(),
                size,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd,
                0,
            )
        };

        unsafe {
            close(fd);
        }
        if addr == MAP_FAILED {
            return Err(anyhow::anyhow!("mmap failed"));
        }

        let shared_pipe = addr as *mut Self;

        if create {
            unsafe {
                (*shared_pipe).init()?;
            }
        }

        Ok(shared_pipe)
    }

    unsafe fn init(&mut self) -> Result<()> {
        let mut attr: pthread_mutexattr_t = unsafe { mem::zeroed() };
        unsafe {
            pthread_mutexattr_init(&mut attr);
            pthread_mutexattr_setpshared(&mut attr, PTHREAD_PROCESS_SHARED);
            pthread_mutexattr_setrobust(&mut attr, PTHREAD_MUTEX_ROBUST);

            if pthread_mutex_init(&mut self.write_mutex, &attr) != 0 {
                return Err(anyhow::anyhow!("Failed to initialize write mutex"));
            }

            if pthread_mutex_init(&mut self.read_mutex, &attr) != 0 {
                return Err(anyhow::anyhow!("Failed to initialize read mutex"));
            }
        }

        self.write_pointer = 0;
        self.read_pointer = 0;
        self.seq = AtomicU64::new(1);
        self.begin = AtomicBool::new(false);
        self.shared_value = AtomicU32::new(0);

        for slot in self.slots.iter_mut() {
            slot.state = AtomicU32::new(SlotState::EMPTY as u32);
            slot.request_id = 0;
            slot.data_size = 0;
            slot.checksum = 0;
            slot.data = [0; SLOT_SIZE];
        }

        Ok(())
    }

    /// 非阻塞抢占slot，如果队列满立即返回错误
    pub unsafe fn hold(&mut self) -> Option<usize> {
        let result = unsafe { pthread_mutex_lock(&mut self.write_mutex) };
        if result == EOWNERDEAD {
            unsafe {
                pthread_mutex_consistent(&mut self.write_mutex);
            }
        } else if result != 0 {
            return None;
        }

        let mut index = None;
        let start_index = self.write_pointer;

        for i in 0..N {
            let slot_index = (start_index + i) % N;
            let slot = &self.slots[slot_index];

            // 简单的状态检查，无需复杂的原子操作
            if slot.state.load(Ordering::Acquire) == SlotState::EMPTY as u32 {
                slot.state
                    .store(SlotState::WRITING as u32, Ordering::Release);
                self.write_pointer = (slot_index + 1) % N;
                index = Some(slot_index);
                break;
            }
        }

        unsafe {
            pthread_mutex_unlock(&mut self.write_mutex);
        }

        index
    }

    /// 向指定索引的槽位写入数据
    pub unsafe fn write<T: bincode::Encode>(&mut self, index: usize, data: &T) -> Result<u64> {
        if index >= N {
            return Err(anyhow::anyhow!("Slot index out of bounds"));
        }

        let slot = &mut self.slots[index];

        // 验证槽位状态
        if slot.state.load(Ordering::Acquire) != SlotState::INPROGRESS as u32 {
            return Err(anyhow::anyhow!("Slot not ready for writing"));
        }

        // 序列化数据
        let serialized = bincode::encode_to_vec(data, bincode::config::standard())
            .map_err(|_| anyhow::anyhow!("Serialization failed"))?;

        if serialized.len() > slot.data.len() {
            return Err(anyhow::anyhow!("Serialized data too large for slot"));
        }

        // 计算校验和
        let checksum = Self::calculate_checksum(&serialized);

        // 更新槽位数据
        slot.data[..serialized.len()].copy_from_slice(&serialized);
        slot.data_size = serialized.len() as u32;
        slot.checksum = checksum;
        slot.request_id = self.seq.fetch_add(1, Ordering::Relaxed);

        // 标记为就绪
        slot.state.store(SlotState::READY as u32, Ordering::Release);

        // 设置"有数据"标志（原子操作，立即对其他进程可见）
        self.begin.store(true, Ordering::SeqCst);

        Ok(slot.request_id)
    }

    /// 获取READY的 slot, 返回index
    pub unsafe fn fetch(&mut self) -> Option<usize> {
        let mut index = None;

        loop {
            // 检查是否有数据（原子操作，非阻塞）
            if self.begin.load(Ordering::SeqCst) {
                let result = unsafe { pthread_mutex_lock(&mut self.read_mutex) };
                if result == EOWNERDEAD {
                    // 内联恢复逻辑
                    unsafe {
                        pthread_mutex_consistent(&mut self.read_mutex);
                    }
                } else if result != 0 {
                    return None;
                }

                let start_index = self.read_pointer;
                for i in 0..N {
                    let slot_index = (start_index + i) % N;
                    let slot = &mut self.slots[slot_index];

                    if slot.state.load(Ordering::Acquire) == SlotState::READY as u32 {
                        slot.state
                            .store(SlotState::READING as u32, Ordering::Release);
                        self.read_pointer = (slot_index + 1) % N;
                        index = Some(slot_index);
                        break;
                    }
                }

                if index.is_none() {
                    // 数据取完，设置"无数据"标志
                    self.begin.store(false, Ordering::SeqCst);
                }

                unsafe {
                    pthread_mutex_unlock(&mut self.read_mutex);
                }
                break;
            } else {
                // 短暂休眠，避免忙等
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        index
    }

    ///  获取 slot 的 data
    /// 并释放 slot 为 EMPTY
    pub unsafe fn read<T: bincode::Decode<()>>(
        &mut self,
        index: usize,
    ) -> Result<Option<(u64, T)>> {
        if index >= N {
            return Err(anyhow::anyhow!("Slot index out of bounds"));
        }

        let slot = &mut self.slots[index];

        // 验证槽位状态
        if slot.state.load(Ordering::Acquire) != SlotState::INPROGRESS as u32 {
            return Err(anyhow::anyhow!("Slot not ready for reading"));
        }

        let mut result_data = None;
        let request_id = slot.request_id;

        // 验证校验和
        let data_slice = &slot.data[..slot.data_size as usize];
        let expected_checksum = Self::calculate_checksum(data_slice);

        if slot.checksum != expected_checksum {
            // 验证校验和失败
            // 清空slot
            slot.state.store(SlotState::EMPTY as u32, Ordering::Release);
            unsafe { pthread_mutex_unlock(&mut self.read_mutex) };
            return Err(anyhow::anyhow!("Checksum mismatch"));
        }

        // 反序列化数据
        match bincode::decode_from_slice::<T, _>(data_slice, bincode::config::standard()) {
            Ok((data, _)) => {
                result_data = Some((request_id, data));

                // 重置slot
                slot.data_size = 0;
                slot.checksum = 0;
                slot.request_id = 0;
                slot.data = [0; SLOT_SIZE];
                slot.state.store(SlotState::EMPTY as u32, Ordering::Release);
            }
            Err(_) => {
                slot.state.store(SlotState::EMPTY as u32, Ordering::Release);
                unsafe {
                    pthread_mutex_unlock(&mut self.read_mutex);
                }
                return Err(anyhow::anyhow!("Deserialization failed"));
            }
        }

        unsafe {
            pthread_mutex_unlock(&mut self.read_mutex);
        }
        Ok(result_data)
    }

    /// 查找第一个 EMPTY 状态的槽位索引
    pub unsafe fn next_empty(&self, current_index: usize) -> Option<usize> {
        unsafe { self.next_slot_by_state(current_index, SlotState::EMPTY) }
    }

    pub unsafe fn next_ready(&self, current_index: usize) -> Option<usize> {
        unsafe { self.next_slot_by_state(current_index, SlotState::READY) }
    }

    /// 查找第一个指定状态的槽位索引
    pub unsafe fn next_slot_by_state(
        &self,
        current_index: usize,
        target_state: SlotState,
    ) -> Option<usize> {
        let mut index = (current_index + 1) % N; // 从下一个位置开始
        // 最多遍历 n 次（覆盖整个数组）
        while index != current_index {
            if self.slots[index].state.load(Ordering::Acquire) == target_state as u32 {
                return Some(index);
            }
            index = (index + 1) % N; // 循环移动到下一个位置
        }
        None
    }

    /// 从指定索引的槽位读取数据（用于新架构）

    /// 获取队列容量
    pub fn capacity(&self) -> usize {
        N
    }

    /// 设置指定索引槽位的状态
    pub unsafe fn set_slot_state(&mut self, index: usize, state: SlotState) -> Result<()> {
        if index >= N {
            return Err(anyhow::anyhow!("Slot index out of bounds"));
        }
        self.slots[index]
            .state
            .store(state as u32, Ordering::Release);
        Ok(())
    }

    /// 获取指定索引槽位的状态
    pub unsafe fn get_slot_state(&self, index: usize) -> Result<SlotState> {
        if index >= N {
            return Err(anyhow::anyhow!("Slot index out of bounds"));
        }
        let state_value = self.slots[index].state.load(Ordering::Acquire);
        match state_value {
            x if x == SlotState::EMPTY as u32 => Ok(SlotState::EMPTY),
            x if x == SlotState::WRITING as u32 => Ok(SlotState::WRITING),
            x if x == SlotState::INPROGRESS as u32 => Ok(SlotState::INPROGRESS),
            x if x == SlotState::READING as u32 => Ok(SlotState::READING),
            x if x == SlotState::READY as u32 => Ok(SlotState::READY),
            _ => Err(anyhow::anyhow!("Unknown slot state: {}", state_value)), // 未知状态
        }
    }

    /// 计算校验和
    fn calculate_checksum(data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }
}
