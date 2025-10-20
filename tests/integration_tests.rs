use mi7soft::shared_memory::*;
use mi7soft::locks::*;
use mi7soft::utils::*;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_mmap_shared_memory() {
    let mut mem = MmapSharedMemory::new(1024).unwrap();
    
    // 测试写入
    let data = b"Hello, World!";
    mem.write(0, data).unwrap();
    
    // 测试读取
    let read_data = mem.read(0, data.len()).unwrap();
    assert_eq!(data, &read_data[..]);
    
    // 测试边界检查
    assert!(mem.write(1020, data).is_err()); // 超出边界
}

#[test]
fn test_shared_memory_manager() {
    let mut manager = SharedMemoryManager::new();
    
    let mem1 = Arc::new(MmapSharedMemory::new(1024).unwrap());
    let mem2 = Arc::new(MmapSharedMemory::new(2048).unwrap());
    
    manager.add_memory(mem1.clone());
    manager.add_memory(mem2.clone());
    
    assert_eq!(manager.count(), 2);
    assert_eq!(manager.get(0).unwrap().size(), 1024);
    assert_eq!(manager.get(1).unwrap().size(), 2048);
}

#[test]
fn test_std_mutex_wrapper() {
    let mutex = Arc::new(StdMutexWrapper::new(0u64));
    let mutex_clone = Arc::clone(&mutex);
    
    let handle = thread::spawn(move || {
        for _ in 0..1000 {
            if let Ok(mut guard) = mutex_clone.lock() {
                *guard += 1;
            }
        }
    });
    
    for _ in 0..1000 {
        if let Ok(mut guard) = mutex.lock() {
            *guard += 1;
        }
    }
    
    handle.join().unwrap();
    
    let final_value = mutex.lock().unwrap();
    assert_eq!(*final_value, 2000);
    
    let stats = mutex.get_stats();
    // 锁统计可能包含额外的锁操作（如最终的读取），所以使用>=
    assert!(stats.lock_count >= 2000);
}

#[test]
fn test_parking_mutex_wrapper() {
    let mutex = Arc::new(ParkingMutexWrapper::new(0u64));
    let mutex_clone = Arc::clone(&mutex);
    
    let handle = thread::spawn(move || {
        for _ in 0..1000 {
            let mut guard = mutex_clone.lock();
            *guard += 1;
        }
    });
    
    for _ in 0..1000 {
        let mut guard = mutex.lock();
        *guard += 1;
    }
    
    handle.join().unwrap();
    
    let final_value = mutex.lock();
    assert_eq!(*final_value, 2000);
    
    let stats = mutex.get_stats();
    // 锁统计可能包含额外的锁操作（如最终的读取），所以使用>=
    assert!(stats.lock_count >= 2000);
}

#[test]
fn test_atomic_counter() {
    let counter = Arc::new(AtomicCounter::new(0));
    let counter_clone = Arc::clone(&counter);
    
    let handle = thread::spawn(move || {
        for _ in 0..1000 {
            counter_clone.increment();
        }
    });
    
    for _ in 0..1000 {
        counter.increment();
    }
    
    handle.join().unwrap();
    
    assert_eq!(counter.get(), 2000);
    
    let stats = counter.get_stats();
    assert_eq!(stats.lock_count, 2000);
}

#[test]
fn test_spin_lock() {
    let lock = Arc::new(SpinLock::new(0u64));
    let lock_clone = Arc::clone(&lock);
    
    let handle = thread::spawn(move || {
        for _ in 0..100 {
            let mut guard = lock_clone.lock();
            *guard += 1;
        }
    });
    
    for _ in 0..100 {
        let mut guard = lock.lock();
        *guard += 1;
    }
    
    handle.join().unwrap();
    
    let final_value = lock.lock();
    assert_eq!(*final_value, 200);
    
    let stats = lock.get_stats();
    // 锁统计可能包含额外的锁操作（如最终的读取），所以使用>=
    assert!(stats.lock_count >= 200);
}

#[test]
fn test_rwlock_wrapper() {
    let rwlock = Arc::new(StdRwLockWrapper::new(vec![0u64; 100]));
    
    // 测试并发读取
    let read_handles: Vec<_> = (0..4).map(|_| {
        let rwlock = Arc::clone(&rwlock);
        thread::spawn(move || {
            for _ in 0..10 {
                let data = rwlock.read().unwrap();
                let _sum: u64 = data.iter().sum();
            }
        })
    }).collect();
    
    // 测试写入
    let write_handle = {
        let rwlock = Arc::clone(&rwlock);
        thread::spawn(move || {
            for i in 0..5 {
                let mut data = rwlock.write().unwrap();
                for item in data.iter_mut() {
                    *item = i;
                }
            }
        })
    };
    
    for handle in read_handles {
        handle.join().unwrap();
    }
    write_handle.join().unwrap();
    
    let stats = rwlock.get_stats();
    assert!(stats.lock_count > 0);
}

#[test]
fn test_performance_tester() {
    let mut tester = PerformanceTester::new();
    
    tester.start();
    thread::sleep(Duration::from_millis(10));
    let duration = tester.stop().unwrap();
    
    assert!(duration >= Duration::from_millis(10));
    assert!(tester.get_average().is_some());
    assert!(tester.get_min().is_some());
    assert!(tester.get_max().is_some());
    
    tester.clear();
    assert!(tester.get_average().is_none());
}

#[test]
fn test_data_generator() {
    let random_data = DataGenerator::random_bytes(100);
    assert_eq!(random_data.len(), 100);
    
    let test_string = DataGenerator::test_string(50);
    assert_eq!(test_string.len(), 50);
    
    let numbers = DataGenerator::number_sequence(10);
    assert_eq!(numbers, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    
    let pattern = DataGenerator::pattern_data(&[1, 2, 3], 10);
    assert_eq!(pattern, vec![1, 2, 3, 1, 2, 3, 1, 2, 3, 1]);
}

#[test]
fn test_formatter() {
    assert_eq!(Formatter::format_bytes(1024), "1.00 KB");
    assert_eq!(Formatter::format_bytes(1048576), "1.00 MB");
    assert_eq!(Formatter::format_bytes(500), "500 B");
    
    let duration = Duration::from_millis(1500);
    let formatted = Formatter::format_duration(duration);
    assert!(formatted.contains("1.50 s"));
    
    assert_eq!(Formatter::format_percentage(25.0, 100.0), "25.00%");
    assert_eq!(Formatter::format_percentage(0.0, 100.0), "0.00%");
}

#[test]
fn test_concurrent_shared_memory_access() {
    let shared_mem = Arc::new(parking_lot::Mutex::new(
        MmapSharedMemory::new(4096).unwrap()
    ));
    
    let handles: Vec<_> = (0..8).map(|thread_id| {
        let shared_mem = Arc::clone(&shared_mem);
        thread::spawn(move || {
            for i in 0..50 {
                let mut mem = shared_mem.lock();
                let data = format!("Thread {} - {}", thread_id, i);
                let offset = (thread_id * 500) as usize;
                
                if offset + data.len() < mem.size() {
                    mem.write(offset, data.as_bytes()).unwrap();
                    
                    // 验证写入
                    let read_data = mem.read(offset, data.len()).unwrap();
                    let read_string = String::from_utf8_lossy(&read_data);
                    assert_eq!(data, read_string);
                }
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_lock_try_operations() {
    // 测试Mutex try_lock
    let mutex = StdMutexWrapper::new(42u64);
    let guard1 = mutex.try_lock();
    assert!(guard1.is_some());
    
    // 测试RwLock try_read/try_write
    let rwlock = StdRwLockWrapper::new(vec![1, 2, 3]);
    let read_guard = rwlock.try_read();
    assert!(read_guard.is_some());
    
    // 测试SpinLock try_lock
    let spin_lock = SpinLock::new(100u64);
    let spin_guard = spin_lock.try_lock();
    assert!(spin_guard.is_some());
}

#[test]
fn test_atomic_compare_and_swap() {
    let counter = AtomicCounter::new(10);
    
    // 成功的CAS
    let result = counter.compare_and_swap(10, 20);
    assert_eq!(result, Ok(10));
    assert_eq!(counter.get(), 20);
    
    // 失败的CAS
    let result = counter.compare_and_swap(10, 30);
    assert_eq!(result, Err(20));
    assert_eq!(counter.get(), 20);
}