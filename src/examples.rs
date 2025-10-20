use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use rayon::prelude::*;
use tokio::task;
use crate::shared_memory::*;
use crate::locks::*;
use crate::Result;

/// 多线程共享内存示例
pub fn multithreaded_shared_memory_example() -> Result<()> {
    println!("=== 多线程共享内存示例 ===");
    
    // 创建共享内存
    let shared_mem = MmapSharedMemory::new(1024)?;
    let shared_mem = Arc::new(parking_lot::Mutex::new(shared_mem));
    
    // 创建多个线程来并发访问共享内存
    let handles: Vec<_> = (0..4).map(|i| {
        let thread_id = i;
        let shared_mem = Arc::clone(&shared_mem);
        
        thread::spawn(move || {
            let mut memory = shared_mem.lock();
            let data = format!("Thread {} data", thread_id);
            if let Err(e) = memory.write(thread_id * 10, data.as_bytes()) {
                eprintln!("写入错误: {}", e);
            }
            
            // 读取数据
            match memory.read(0, 20) {
                Ok(data) => println!("线程 {} 读取到: {:?}", thread_id, String::from_utf8_lossy(&data)),
                Err(e) => eprintln!("读取错误: {}", e),
            }
        })
    }).collect();
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    println!("多线程共享内存示例完成\n");
    Ok(())
}

/// 锁性能比较示例
pub fn lock_performance_comparison() -> Result<()> {
    println!("=== 锁性能比较示例 ===");
    
    const ITERATIONS: usize = 100_000;
    const THREAD_COUNT: usize = 4;
    
    // 测试标准库Mutex
    let std_mutex = Arc::new(StdMutexWrapper::new(0u64));
    let start = Instant::now();
    
    let handles: Vec<_> = (0..THREAD_COUNT).map(|_| {
        let mutex = Arc::clone(&std_mutex);
        thread::spawn(move || {
            for _ in 0..ITERATIONS / THREAD_COUNT {
                if let Ok(mut guard) = mutex.lock() {
                    *guard += 1;
                }
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let std_duration = start.elapsed();
    let std_stats = std_mutex.get_stats();
    println!("标准库Mutex: {:?}, 统计: {:?}", std_duration, std_stats);
    
    // 测试Parking Lot Mutex
    let parking_mutex = Arc::new(ParkingMutexWrapper::new(0u64));
    let start = Instant::now();
    
    let handles: Vec<_> = (0..THREAD_COUNT).map(|_| {
        let mutex = Arc::clone(&parking_mutex);
        thread::spawn(move || {
            for _ in 0..ITERATIONS / THREAD_COUNT {
                let mut guard = mutex.lock();
                *guard += 1;
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let parking_duration = start.elapsed();
    let parking_stats = parking_mutex.get_stats();
    println!("Parking Lot Mutex: {:?}, 统计: {:?}", parking_duration, parking_stats);
    
    // 测试原子操作
    let atomic_counter = Arc::new(AtomicCounter::new(0));
    let start = Instant::now();
    
    let handles: Vec<_> = (0..THREAD_COUNT).map(|_| {
        let counter = Arc::clone(&atomic_counter);
        thread::spawn(move || {
            for _ in 0..ITERATIONS / THREAD_COUNT {
                counter.increment();
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let atomic_duration = start.elapsed();
    let atomic_stats = atomic_counter.get_stats();
    println!("原子操作: {:?}, 统计: {:?}", atomic_duration, atomic_stats);
    println!("最终计数值: {}", atomic_counter.get());
    
    // 测试自旋锁
    let spin_lock = Arc::new(SpinLock::new(0u64));
    let start = Instant::now();
    
    let handles: Vec<_> = (0..THREAD_COUNT).map(|_| {
        let lock = Arc::clone(&spin_lock);
        thread::spawn(move || {
            for _ in 0..ITERATIONS / THREAD_COUNT {
                let mut guard = lock.lock();
                *guard += 1;
            }
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let spin_duration = start.elapsed();
    let spin_stats = spin_lock.get_stats();
    println!("自旋锁: {:?}, 统计: {:?}", spin_duration, spin_stats);
    
    println!("锁性能比较完成\n");
    Ok(())
}

/// 读写锁示例
pub fn rwlock_example() -> Result<()> {
    println!("=== 读写锁示例 ===");
    
    let rwlock = Arc::new(StdRwLockWrapper::new(vec![0u64; 1000]));
    
    // 创建多个读线程
    let read_handles: Vec<_> = (0..8).map(|reader_id| {
        let rwlock = Arc::clone(&rwlock);
        thread::spawn(move || {
            for i in 0..50 {
                if let Ok(data) = rwlock.read() {
                    let sum: u64 = data.iter().sum();
                    println!("读线程 {} 第 {} 次读取，数组和: {}", reader_id, i, sum);
                }
                thread::sleep(Duration::from_millis(10));
            }
        })
    }).collect();
    
    // 创建少量写线程
    let write_handles: Vec<_> = (0..2).map(|writer_id| {
        let rwlock = Arc::clone(&rwlock);
        thread::spawn(move || {
            for i in 0..20 {
                if let Ok(mut data) = rwlock.write() {
                    for item in data.iter_mut() {
                        *item += 1;
                    }
                    println!("写线程 {} 第 {} 次写入完成", writer_id, i);
                }
                thread::sleep(Duration::from_millis(50));
            }
        })
    }).collect();
    
    // 等待所有线程完成
    for handle in read_handles {
        handle.join().unwrap();
    }
    for handle in write_handles {
        handle.join().unwrap();
    }
    
    let stats = rwlock.get_stats();
    println!("读写锁统计: {:?}", stats);
    println!("读写锁示例完成\n");
    Ok(())
}

/// 使用Rayon的并行计算示例
pub fn rayon_parallel_example() -> Result<()> {
    println!("=== Rayon并行计算示例 ===");
    
    let shared_mem = Arc::new(parking_lot::Mutex::new(MmapSharedMemory::new(8192)?));
    
    // 并行初始化共享内存
    (0..1000u32).into_par_iter().for_each(|i| {
        let mut mem = shared_mem.lock();
        let data = i.to_le_bytes();
        let offset = (i as usize) * 4;
        
        if offset + 4 <= mem.size() {
            if let Err(e) = mem.write(offset, &data) {
                eprintln!("写入错误 at {}: {}", offset, e);
            }
        }
    });
    
    // 并行读取和验证
    let results: Vec<_> = (0..1000u32).into_par_iter().map(|i| {
        let mem = shared_mem.lock();
        let offset = (i as usize) * 4;
        
        if offset + 4 <= mem.size() {
            if let Ok(data) = mem.read(offset, 4) {
                let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                return value == i;
            }
        }
        false
    }).collect();
    
    let correct_count = results.iter().filter(|&&x| x).count();
    println!("Rayon并行验证: {}/{} 正确", correct_count, results.len());
    println!("Rayon并行计算示例完成\n");
    Ok(())
}

/// 异步Tokio示例
pub async fn tokio_async_example() -> Result<()> {
    println!("=== Tokio异步示例 ===");
    
    let shared_mem = Arc::new(tokio::sync::Mutex::new(MmapSharedMemory::new(4096)?));
    
    // 创建多个异步任务
    let tasks: Vec<_> = (0..10).map(|task_id| {
        let shared_mem = Arc::clone(&shared_mem);
        task::spawn(async move {
            for i in 0..20 {
                let mut mem = shared_mem.lock().await;
                let data = format!("Task {} - Iteration {}", task_id, i);
                let offset = (task_id * 200) as usize;
                
                if let Err(e) = mem.write(offset, data.as_bytes()) {
                    eprintln!("异步写入错误: {}", e);
                    continue;
                }
                
                println!("异步任务 {} 完成第 {} 次写入", task_id, i);
                drop(mem); // 释放锁
                
                // 异步等待
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
    }).collect();
    
    // 等待所有任务完成
    for task in tasks {
        task.await.unwrap();
    }
    
    println!("Tokio异步示例完成\n");
    Ok(())
}

/// 跨进程共享内存示例（需要手动运行多个进程实例）
pub fn cross_process_example() -> Result<()> {
    println!("=== 跨进程共享内存示例 ===");
    
    // 尝试创建共享内存
    match CrossProcessSharedMemory::create("demo_shared_mem", 4096) {
        Ok(mut shared_mem) => {
            println!("成功创建共享内存，OS ID: {}", shared_mem.get_os_id());
            
            // 写入一些数据
            let message = "Hello from process creator!";
            shared_mem.write(0, message.as_bytes())?;
            
            println!("已写入消息: {}", message);
            println!("共享内存已创建，其他进程可以通过相同的名称访问");
            
            // 等待一段时间让其他进程访问
            thread::sleep(Duration::from_secs(10));
            
            // 读取可能被其他进程修改的数据
            if let Ok(data) = shared_mem.read(0, 100) {
                let received = String::from_utf8_lossy(&data);
                println!("从共享内存读取: {}", received.trim_end_matches('\0'));
            }
        }
        Err(_) => {
            // 尝试打开已存在的共享内存
            match CrossProcessSharedMemory::open("demo_shared_mem") {
                Ok(mut shared_mem) => {
                    println!("成功打开已存在的共享内存");
                    
                    // 读取数据
                    if let Ok(data) = shared_mem.read(0, 100) {
                        let received = String::from_utf8_lossy(&data);
                        println!("从共享内存读取: {}", received.trim_end_matches('\0'));
                    }
                    
                    // 写入回复
                    let reply = "Hello from process reader!";
                    shared_mem.write(0, reply.as_bytes())?;
                    println!("已写入回复: {}", reply);
                }
                Err(e) => {
                    println!("无法创建或打开共享内存: {}", e);
                    println!("请先运行一个进程实例来创建共享内存");
                }
            }
        }
    }
    
    println!("跨进程共享内存示例完成\n");
    Ok(())
}

/// 运行所有示例
pub async fn run_all_examples() -> Result<()> {
    println!("开始运行所有共享内存和锁机制示例...\n");
    
    multithreaded_shared_memory_example()?;
    lock_performance_comparison()?;
    rwlock_example()?;
    rayon_parallel_example()?;
    tokio_async_example().await?;
    cross_process_example()?;
    
    println!("所有示例运行完成！");
    Ok(())
}