use libc::{
    EOWNERDEAD, MAP_FAILED, MAP_SHARED, O_CREAT, O_RDWR, PROT_READ, PROT_WRITE,
    PTHREAD_MUTEX_ROBUST, PTHREAD_PROCESS_SHARED, ftruncate, mmap, pthread_mutex_consistent,
    pthread_mutex_init, pthread_mutex_lock, pthread_mutex_t, pthread_mutex_unlock,
    pthread_mutexattr_init, pthread_mutexattr_setpshared, pthread_mutexattr_setrobust,
    pthread_mutexattr_t,
};

use std::{ffi::CString, mem, ptr};

use std::sync::atomic::{AtomicU64, Ordering};

use nix::sys::eventfd;
use tokio::io::unix::AsyncFd;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SlotState {
    EMPTY = 0,
    PENDINGWRITE = 1,
    INPROGRESS = 2,
    PENDINGREAD = 3,
    FULL = 4,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Slot<const SLOT_SIZE: usize> {
    pub state: SlotState,
    pub request_id: u64,
    pub data: [u8; SLOT_SIZE],
}

#[repr(C)]
pub struct SharedSlotPipe<const N: usize, const SLOT_SIZE: usize> {
    pub write_mutex: pthread_mutex_t, // 保护写操作
    pub read_mutex: pthread_mutex_t,  // 保护读操作
    pub write_pointer: usize,         // 可写的索引
    pub read_pointer: usize,          // 可读的索引
    pub slots: [Slot<SLOT_SIZE>; N],
    pub seq: AtomicU64, // request_id 生成器
}

unsafe impl<const N: usize, const SLOT_SIZE: usize> Send for SharedSlotPipe<N, SLOT_SIZE> {}

unsafe impl<const N: usize, const SLOT_SIZE: usize> Sync for SharedSlotPipe<N, SLOT_SIZE> {}

impl<const N: usize, const SLOT_SIZE: usize> SharedSlotPipe<N, SLOT_SIZE> {
    /// 创建或连接共享内存
    pub unsafe fn open(name: &str, create: bool) -> *mut Self {
        let cname = CString::new(name).unwrap();
        let flags = if create { O_CREAT | O_RDWR } else { O_RDWR };
        let fd = libc::shm_open(cname.as_ptr(), flags, 0o666);
        assert!(fd >= 0, "shm_open failed");

        let size = mem::size_of::<Self>();
        ftruncate(fd, size as i64);

        let addr = mmap(
            ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            fd,
            0,
        );
        if addr == MAP_FAILED {
            panic!("mmap failed");
        }

        let share_slot = addr as *mut Self;

        if create {
            (*share_slot).init();
        }

        share_slot
    }

    unsafe fn init(&mut self) {
        let mut attr: pthread_mutexattr_t = mem::zeroed();
        pthread_mutexattr_init(&mut attr);
        pthread_mutexattr_setpshared(&mut attr, PTHREAD_PROCESS_SHARED);
        pthread_mutexattr_setrobust(&mut attr, PTHREAD_MUTEX_ROBUST);

        // 初始化两个独立的互斥锁
        pthread_mutex_init(&mut self.write_mutex, &attr);
        pthread_mutex_init(&mut self.read_mutex, &attr);

        self.write_pointer = 0;
        self.read_pointer = 0;
        self.seq = AtomicU64::new(1);

        for slot in self.slots.iter_mut() {
            slot.state = SlotState::EMPTY;
            slot.request_id = 0;
            slot.data = [0; SLOT_SIZE];
        }
    }

    /// 非阻塞抢占slot，如果队列满立即返回错误
    pub unsafe fn hold(&mut self) -> Option<usize> {
        let result = pthread_mutex_lock(&mut self.write_mutex);
        if result == EOWNERDEAD {
            pthread_mutex_consistent(&mut self.write_mutex);
        } else if result != 0 {
            // Failed to lock tail mutex
            return None;
        }

        // 检查队列是否满，如果满立即返回错误
        if self.slots[self.write_pointer].state != SlotState::EMPTY {
            if let Some(empty_index) = self.next_empty(self.write_pointer) {
                // 找到了其他空槽位，可以更新 write_pointer
                self.write_pointer = empty_index;
            } else {
                // 真正满了，无法找到空槽位
                pthread_mutex_unlock(&mut self.write_mutex);
                return None;
            }
        }

        let slot = &mut self.slots[self.write_pointer];
        slot.state = SlotState::PENDINGWRITE;

        let index = self.write_pointer;

        self.write_pointer = (self.write_pointer + 1) % N;

        pthread_mutex_unlock(&mut self.write_mutex);

        Some(index)
    }

    /// 向指定索引的槽位写入数据
    pub unsafe fn store<T: bincode::Encode>(
        &mut self,
        index: usize,
        value: &T,
    ) -> Result<u64, &'static str> {
        if index >= N {
            return Err("索引超出范围");
        }

        let slot = &mut self.slots[index];

        // 检查槽位状态是否为 PENDINGWRITE
        if slot.state != SlotState::PENDINGWRITE {
            return Err("槽位状态不是 PENDINGWRITE");
        }

        // 设置为 INPROGRESS
        slot.state = SlotState::INPROGRESS;
        slot.request_id = self.seq.fetch_add(1, Ordering::SeqCst);

        // 序列化数据
        let encoded = bincode::encode_to_vec(value, bincode::config::standard()).unwrap();
        let len = encoded.len().min(SLOT_SIZE);
        slot.data[..len].copy_from_slice(&encoded[..len]);

        // 设置为 FULL
        slot.state = SlotState::FULL;

        Ok(slot.request_id)
    }

    /// 获取FULL的 slot, 返回index
    pub unsafe fn fetch(&mut self) -> Option<usize> {
        let result = pthread_mutex_lock(&mut self.read_mutex);
        if result == EOWNERDEAD {
            // 内联恢复逻辑
            pthread_mutex_consistent(&mut self.read_mutex);
        } else if result != 0 {
            return None;
        }

        // 检查队列是否为空，如果为空立即返回
        if self.slots[self.read_pointer].state != SlotState::FULL {
            if let Some(full_index) = self.next_full(self.read_pointer) {
                // 找到了其他非空的槽位，可以更新 read_pointer
                self.read_pointer = full_index;
            } else {
                // 队列为空，无法找到非空的槽位
                pthread_mutex_unlock(&mut self.read_mutex);
                return None;
            }
        }

        let slot = &mut self.slots[self.read_pointer];
        slot.state = SlotState::PENDINGREAD;

        let index = self.read_pointer;

        self.read_pointer = (self.read_pointer + 1) % N;

        pthread_mutex_unlock(&mut self.read_mutex);

        Some(index)
    }

    ///  获取 slot 的 data
    /// 并释放 slot 为 EMPTY
    pub unsafe fn release<T: bincode::Decode<()>>(
        &mut self,
        index: usize,
    ) -> Result<Option<(u64, T)>, &'static str> {
        if index >= N {
            return Err("索引超出范围");
        }

        let slot = &mut self.slots[index];

        // 检查槽位状态是否为 PENDINGREAD
        if slot.state != SlotState::PENDINGREAD {
            return Err("槽位状态不是 PENDINGREAD");
        }

        // 设置为 INPROGRESS
        slot.state = SlotState::INPROGRESS;
        let id = slot.request_id;

        // 反序列化数据
        let data = bincode::decode_from_slice::<T, _>(&slot.data, bincode::config::standard())
            .ok()
            .map(|(v, _)| v);

        // 重置slot
        slot.state = SlotState::EMPTY;
        slot.request_id = 0;
        slot.data = [0; SLOT_SIZE];

        Ok(data.map(|v| (id, v)))
    }

    /// 查找第一个 EMPTY 状态的槽位索引
    pub unsafe fn next_empty(&self, current_index: usize) -> Option<usize> {
        self.next_slot_by_state(current_index, SlotState::EMPTY)
    }

    pub unsafe fn next_full(&self, current_index: usize) -> Option<usize> {
        self.next_slot_by_state(current_index, SlotState::FULL)
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
            if self.slots[index].state == target_state {
                return Some(index);
            }
            index = (index + 1) % N; // 循环移动到下一个位置
        }
        None
    }

    /// 设置指定索引槽位的状态
    pub unsafe fn set_slot_state(
        &mut self,
        index: usize,
        state: SlotState,
    ) -> Result<(), &'static str> {
        if index >= N {
            return Err("索引超出范围");
        }
        self.slots[index].state = state;
        Ok(())
    }

    /// 获取指定索引槽位的状态
    pub unsafe fn get_slot_state(&self, index: usize) -> Result<SlotState, &'static str> {
        if index >= N {
            return Err("索引超出范围");
        }
        Ok(self.slots[index].state)
    }

    /// 从指定索引的槽位读取数据（用于新架构）

    /// 获取队列容量
    pub fn capacity(&self) -> usize {
        N
    }
}
