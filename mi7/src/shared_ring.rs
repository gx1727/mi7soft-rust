
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
    pub mutex: pthread_mutex_t,
    pub cond_not_empty: pthread_cond_t,
    pub cond_not_full: pthread_cond_t,
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

        let addr = mmap(ptr::null_mut(), size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
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

        pthread_mutex_init(&mut self.mutex, &attr);
        pthread_cond_init(&mut self.cond_not_empty, ptr::null());
        pthread_cond_init(&mut self.cond_not_full, ptr::null());

        self.head = 0;
        self.tail = 0;
        self.seq = AtomicU64::new(1);

        for slot in self.slots.iter_mut() {
            slot.state = SlotState::EMPTY;
            slot.request_id = 0;
            slot.data = [0; SLOT_SIZE];
        }
    }

    unsafe fn lock(&mut self) {
        let r = pthread_mutex_lock(&mut self.mutex);
        if r == EOWNERDEAD {
            self.recover();
        }
    }

    unsafe fn recover(&mut self) {
        eprintln!("[Recovery] Detected EOWNERDEAD — scanning slots...");
        for (i, slot) in self.slots.iter_mut().enumerate() {
            match slot.state {
                SlotState::INPROGRESS => {
                    eprintln!("slot[{i}] INPROGRESS -> EMPTY");
                    slot.state = SlotState::EMPTY;
                    slot.request_id = 0;
                }
                SlotState::FULL => {}
                SlotState::EMPTY => {}
            }
        }
        pthread_mutex_consistent(&mut self.mutex);
    }

    pub unsafe fn push<T: bincode::Encode>(&mut self, value: &T) -> u64 {
        self.lock();
        while self.slots[self.tail].state != SlotState::EMPTY {
            pthread_cond_wait(&mut self.cond_not_full, &mut self.mutex);
        }

        let slot = &mut self.slots[self.tail];
        slot.state = SlotState::INPROGRESS;
        slot.request_id = self.seq.fetch_add(1, Ordering::SeqCst);

        let encoded = bincode::encode_to_vec(value, bincode::config::standard()).unwrap();
        let len = encoded.len().min(SLOT_SIZE);
        slot.data[..len].copy_from_slice(&encoded[..len]);
        slot.state = SlotState::FULL;

        self.tail = (self.tail + 1) % N;
        pthread_cond_signal(&mut self.cond_not_empty);
        pthread_mutex_unlock(&mut self.mutex);

        slot.request_id
    }

    pub unsafe fn pop<T: bincode::Decode<()>>(&mut self) -> Option<(u64, T)> {
        self.lock();
        while self.slots[self.head].state != SlotState::FULL {
            pthread_cond_wait(&mut self.cond_not_empty, &mut self.mutex);
        }

        let slot = &mut self.slots[self.head];
        let id = slot.request_id;
        let data = bincode::decode_from_slice::<T, _>(&slot.data, bincode::config::standard()).ok().map(|(v, _)| v);

        slot.state = SlotState::EMPTY;
        slot.request_id = 0;
        pthread_cond_signal(&mut self.cond_not_full);
        self.head = (self.head + 1) % N;
        pthread_mutex_unlock(&mut self.mutex);

        data.map(|v| (id, v))
    }
}
