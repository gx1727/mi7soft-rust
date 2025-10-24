use libc::*;

use std::{ffi::CString, mem, ptr};

use std::sync::atomic::{AtomicU64, Ordering};

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SlotState {
    EMPTY = 0,
    INPROGRESS = 1,
    FULL = 2,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Slot<const SLOT_SIZE: usize> {
    pub state: SlotState,
    pub request_id: u64,
    pub data: [u8; SLOT_SIZE],
}

#[repr(C)]
pub struct SharedRingQueue<const N: usize, const SLOT_SIZE: usize> {
    pub head_mutex: pthread_mutex_t, // 保护 head 和读操作
    pub tail_mutex: pthread_mutex_t, // 保护 tail 和写操作
    pub head: usize,
    pub tail: usize,
    pub slots: [Slot<SLOT_SIZE>; N],
    pub seq: AtomicU64, // request_id 生成器
}

unsafe impl<const N: usize, const SLOT_SIZE: usize> Send for SharedRingQueue<N, SLOT_SIZE> {}

unsafe impl<const N: usize, const SLOT_SIZE: usize> Sync for SharedRingQueue<N, SLOT_SIZE> {}

impl<const N: usize, const SLOT_SIZE: usize> SharedRingQueue<N, SLOT_SIZE> {
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

        let ring = addr as *mut Self;

        if create {
            (*ring).init();
        }

        ring
    }

    unsafe fn init(&mut self) {
        let mut attr: pthread_mutexattr_t = mem::zeroed();
        pthread_mutexattr_init(&mut attr);
        pthread_mutexattr_setpshared(&mut attr, PTHREAD_PROCESS_SHARED);
        pthread_mutexattr_setrobust(&mut attr, PTHREAD_MUTEX_ROBUST);

        // 初始化两个独立的互斥锁
        pthread_mutex_init(&mut self.head_mutex, &attr);
        pthread_mutex_init(&mut self.tail_mutex, &attr);

        self.head = 0;
        self.tail = 0;
        self.seq = AtomicU64::new(1);

        for slot in self.slots.iter_mut() {
            slot.state = SlotState::EMPTY;
            slot.request_id = 0;
            slot.data = [0; SLOT_SIZE];
        }
    }

    /// 非阻塞推送消息到队列，如果队列满立即返回错误
    pub unsafe fn push<T: bincode::Encode>(&mut self, value: &T) -> Result<u64, &'static str> {
        let result = pthread_mutex_lock(&mut self.tail_mutex);
        if result == EOWNERDEAD {
            // 内联恢复逻辑：扫描并清理槽位
            for (i, slot) in self.slots.iter_mut().enumerate() {
                if slot.state == SlotState::INPROGRESS {
                    slot.state = SlotState::EMPTY;
                    slot.request_id = 0;
                }
            }
            pthread_mutex_consistent(&mut self.tail_mutex);
        } else if result != 0 {
            return Err("Failed to lock tail mutex");
        }

        // 检查队列是否满，如果满立即返回错误
        if self.slots[self.tail].state != SlotState::EMPTY {
            pthread_mutex_unlock(&mut self.tail_mutex);
            return Err("队列已满");
        }

        let slot = &mut self.slots[self.tail];
        slot.state = SlotState::INPROGRESS;
        slot.request_id = self.seq.fetch_add(1, Ordering::SeqCst);

        let encoded = bincode::encode_to_vec(value, bincode::config::standard()).unwrap();
        let len = encoded.len().min(SLOT_SIZE);
        slot.data[..len].copy_from_slice(&encoded[..len]);
        slot.state = SlotState::FULL;

        self.tail = (self.tail + 1) % N;
        pthread_mutex_unlock(&mut self.tail_mutex);

        Ok(slot.request_id)
    }

    /// 非阻塞从队列弹出消息，如果队列为空立即返回 None
    pub unsafe fn pop<T: bincode::Decode<()>>(&mut self) -> Option<(u64, T)> {
        let result = pthread_mutex_lock(&mut self.head_mutex);
        if result == EOWNERDEAD {
            // 内联恢复逻辑
            pthread_mutex_consistent(&mut self.head_mutex);
        } else if result != 0 {
            return None;
        }

        // 检查队列是否为空，如果为空立即返回
        if self.slots[self.head].state != SlotState::FULL {
            pthread_mutex_unlock(&mut self.head_mutex);
            return None;
        }

        let slot = &mut self.slots[self.head];
        let id = slot.request_id;
        let data = bincode::decode_from_slice::<T, _>(&slot.data, bincode::config::standard())
            .ok()
            .map(|(v, _)| v);

        slot.state = SlotState::EMPTY;
        slot.request_id = 0;
        self.head = (self.head + 1) % N;
        pthread_mutex_unlock(&mut self.head_mutex);

        data.map(|v| (id, v))
    }
}
