use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use parking_lot::{Mutex as ParkingMutex};
use std::time::{Duration, Instant};
use crate::{Result, SharedMemoryError};

/// 锁性能统计
#[derive(Debug, Clone)]
pub struct LockStats {
    pub lock_count: u64,
    pub unlock_count: u64,
    pub total_wait_time: Duration,
    pub max_wait_time: Duration,
    pub contention_count: u64,
}

impl Default for LockStats {
    fn default() -> Self {
        Self {
            lock_count: 0,
            unlock_count: 0,
            total_wait_time: Duration::ZERO,
            max_wait_time: Duration::ZERO,
            contention_count: 0,
        }
    }
}

/// 标准库Mutex包装器，带性能统计
pub struct StdMutexWrapper<T> {
    mutex: Arc<Mutex<T>>,
    stats: Arc<Mutex<LockStats>>,
}

impl<T> StdMutexWrapper<T> {
    pub fn new(data: T) -> Self {
        Self {
            mutex: Arc::new(Mutex::new(data)),
            stats: Arc::new(Mutex::new(LockStats::default())),
        }
    }
    
    pub fn lock(&self) -> Result<std::sync::MutexGuard<T>> {
        let start = Instant::now();
        let guard = self.mutex.lock()
            .map_err(|e| SharedMemoryError::LockFailed(e.to_string()))?;
        let wait_time = start.elapsed();
        
        // 更新统计信息
        if let Ok(mut stats) = self.stats.lock() {
            stats.lock_count += 1;
            stats.total_wait_time += wait_time;
            if wait_time > stats.max_wait_time {
                stats.max_wait_time = wait_time;
            }
            if wait_time > Duration::from_micros(100) {
                stats.contention_count += 1;
            }
        }
        
        Ok(guard)
    }
    
    pub fn try_lock(&self) -> Option<std::sync::MutexGuard<T>> {
        self.mutex.try_lock().ok()
    }
    
    pub fn get_stats(&self) -> LockStats {
        self.stats.lock().unwrap().clone()
    }
}

impl<T> Clone for StdMutexWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            mutex: Arc::clone(&self.mutex),
            stats: Arc::clone(&self.stats),
        }
    }
}

/// 标准库RwLock包装器，带性能统计
pub struct StdRwLockWrapper<T> {
    rwlock: Arc<RwLock<T>>,
    stats: Arc<Mutex<LockStats>>,
}

impl<T> StdRwLockWrapper<T> {
    pub fn new(data: T) -> Self {
        Self {
            rwlock: Arc::new(RwLock::new(data)),
            stats: Arc::new(Mutex::new(LockStats::default())),
        }
    }
    
    pub fn read(&self) -> Result<std::sync::RwLockReadGuard<T>> {
        let start = Instant::now();
        let guard = self.rwlock.read()
            .map_err(|e| SharedMemoryError::LockFailed(e.to_string()))?;
        let wait_time = start.elapsed();
        
        self.update_stats(wait_time);
        Ok(guard)
    }
    
    pub fn write(&self) -> Result<std::sync::RwLockWriteGuard<T>> {
        let start = Instant::now();
        let guard = self.rwlock.write()
            .map_err(|e| SharedMemoryError::LockFailed(e.to_string()))?;
        let wait_time = start.elapsed();
        
        self.update_stats(wait_time);
        Ok(guard)
    }
    
    pub fn try_read(&self) -> Option<std::sync::RwLockReadGuard<T>> {
        self.rwlock.try_read().ok()
    }
    
    pub fn try_write(&self) -> Option<std::sync::RwLockWriteGuard<T>> {
        self.rwlock.try_write().ok()
    }
    
    fn update_stats(&self, wait_time: Duration) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.lock_count += 1;
            stats.total_wait_time += wait_time;
            if wait_time > stats.max_wait_time {
                stats.max_wait_time = wait_time;
            }
            if wait_time > Duration::from_micros(100) {
                stats.contention_count += 1;
            }
        }
    }
    
    pub fn get_stats(&self) -> LockStats {
        self.stats.lock().unwrap().clone()
    }
}

impl<T> Clone for StdRwLockWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            rwlock: Arc::clone(&self.rwlock),
            stats: Arc::clone(&self.stats),
        }
    }
}

/// Parking Lot Mutex包装器
pub struct ParkingMutexWrapper<T> {
    mutex: Arc<ParkingMutex<T>>,
    stats: Arc<Mutex<LockStats>>,
}

impl<T> ParkingMutexWrapper<T> {
    pub fn new(data: T) -> Self {
        Self {
            mutex: Arc::new(ParkingMutex::new(data)),
            stats: Arc::new(Mutex::new(LockStats::default())),
        }
    }
    
    pub fn lock(&self) -> parking_lot::MutexGuard<T> {
        let start = Instant::now();
        let guard = self.mutex.lock();
        let wait_time = start.elapsed();
        
        self.update_stats(wait_time);
        guard
    }
    
    pub fn try_lock(&self) -> Option<parking_lot::MutexGuard<T>> {
        self.mutex.try_lock()
    }
    
    pub fn try_lock_for(&self, timeout: Duration) -> Option<parking_lot::MutexGuard<T>> {
        let start = Instant::now();
        let guard = self.mutex.try_lock_for(timeout);
        if guard.is_some() {
            let wait_time = start.elapsed();
            self.update_stats(wait_time);
        }
        guard
    }
    
    fn update_stats(&self, wait_time: Duration) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.lock_count += 1;
            stats.total_wait_time += wait_time;
            if wait_time > stats.max_wait_time {
                stats.max_wait_time = wait_time;
            }
            if wait_time > Duration::from_micros(100) {
                stats.contention_count += 1;
            }
        }
    }
    
    pub fn get_stats(&self) -> LockStats {
        self.stats.lock().unwrap().clone()
    }
}

impl<T> Clone for ParkingMutexWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            mutex: Arc::clone(&self.mutex),
            stats: Arc::clone(&self.stats),
        }
    }
}

/// 原子操作计数器
pub struct AtomicCounter {
    value: AtomicU64,
    stats: Arc<Mutex<LockStats>>,
}

impl AtomicCounter {
    pub fn new(initial: u64) -> Self {
        Self {
            value: AtomicU64::new(initial),
            stats: Arc::new(Mutex::new(LockStats::default())),
        }
    }
    
    pub fn increment(&self) -> u64 {
        let start = Instant::now();
        let result = self.value.fetch_add(1, Ordering::SeqCst);
        let wait_time = start.elapsed();
        
        self.update_stats(wait_time);
        result + 1
    }
    
    pub fn decrement(&self) -> u64 {
        let start = Instant::now();
        let result = self.value.fetch_sub(1, Ordering::SeqCst);
        let wait_time = start.elapsed();
        
        self.update_stats(wait_time);
        result - 1
    }
    
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::SeqCst)
    }
    
    pub fn set(&self, value: u64) {
        let start = Instant::now();
        self.value.store(value, Ordering::SeqCst);
        let wait_time = start.elapsed();
        
        self.update_stats(wait_time);
    }
    
    pub fn compare_and_swap(&self, current: u64, new: u64) -> std::result::Result<u64, u64> {
        let start = Instant::now();
        let result = self.value.compare_exchange(current, new, Ordering::SeqCst, Ordering::Relaxed);
        
        let wait_time = start.elapsed();
        self.update_stats(wait_time);
        
        result
    }
    
    fn update_stats(&self, wait_time: Duration) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.lock_count += 1;
            stats.total_wait_time += wait_time;
            if wait_time > stats.max_wait_time {
                stats.max_wait_time = wait_time;
            }
        }
    }
    
    pub fn get_stats(&self) -> LockStats {
        self.stats.lock().unwrap().clone()
    }
}

/// 自旋锁实现
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: std::cell::UnsafeCell<T>,
    stats: Arc<Mutex<LockStats>>,
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: std::cell::UnsafeCell::new(data),
            stats: Arc::new(Mutex::new(LockStats::default())),
        }
    }
    
    pub fn lock(&self) -> SpinLockGuard<T> {
        let start = Instant::now();
        let mut spin_count = 0;
        
        while self.locked.compare_exchange_weak(
            false, 
            true, 
            Ordering::Acquire, 
            Ordering::Relaxed
        ).is_err() {
            spin_count += 1;
            if spin_count > 100 {
                std::thread::yield_now();
                spin_count = 0;
            }
        }
        
        let wait_time = start.elapsed();
        self.update_stats(wait_time);
        
        SpinLockGuard { lock: self }
    }
    
    pub fn try_lock(&self) -> Option<SpinLockGuard<T>> {
        if self.locked.compare_exchange(
            false, 
            true, 
            Ordering::Acquire, 
            Ordering::Relaxed
        ).is_ok() {
            Some(SpinLockGuard { lock: self })
        } else {
            None
        }
    }
    
    fn update_stats(&self, wait_time: Duration) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.lock_count += 1;
            stats.total_wait_time += wait_time;
            if wait_time > stats.max_wait_time {
                stats.max_wait_time = wait_time;
            }
            if wait_time > Duration::from_micros(100) {
                stats.contention_count += 1;
            }
        }
    }
    
    pub fn get_stats(&self) -> LockStats {
        self.stats.lock().unwrap().clone()
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> std::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> std::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        
        if let Ok(mut stats) = self.lock.stats.lock() {
            stats.unlock_count += 1;
        }
    }
}