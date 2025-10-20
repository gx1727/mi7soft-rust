use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mi7soft::shared_memory::*;
use mi7soft::locks::*;
use std::sync::Arc;
use std::thread;

fn bench_shared_memory_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("shared_memory_creation");
    
    for size in [1024, 4096, 16384, 65536].iter() {
        group.bench_with_input(BenchmarkId::new("mmap", size), size, |b, &size| {
            b.iter(|| {
                let mem = MmapSharedMemory::new(size);
                black_box(mem)
            })
        });
    }
    
    group.finish();
}

fn bench_shared_memory_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("shared_memory_write");
    
    let mut mem = MmapSharedMemory::new(65536).unwrap();
    let data = vec![0u8; 1024];
    
    group.bench_function("write_1kb", |b| {
        b.iter(|| {
            mem.write(black_box(0), black_box(&data)).unwrap()
        })
    });
    
    group.finish();
}

fn bench_shared_memory_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("shared_memory_read");
    
    let mut mem = MmapSharedMemory::new(65536).unwrap();
    let data = vec![42u8; 1024];
    mem.write(0, &data).unwrap();
    
    group.bench_function("read_1kb", |b| {
        b.iter(|| {
            let result = mem.read(black_box(0), black_box(1024));
            black_box(result)
        })
    });
    
    group.finish();
}

fn bench_mutex_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("mutex_contention");
    
    // 标准库Mutex
    group.bench_function("std_mutex", |b| {
        let mutex = Arc::new(StdMutexWrapper::new(0u64));
        b.iter(|| {
            let handles: Vec<_> = (0..4).map(|_| {
                let mutex = Arc::clone(&mutex);
                thread::spawn(move || {
                    for _ in 0..100 {
                        if let Ok(mut guard) = mutex.lock() {
                            *guard += 1;
                        }
                    }
                })
            }).collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
    
    // Parking Lot Mutex
    group.bench_function("parking_mutex", |b| {
        let mutex = Arc::new(ParkingMutexWrapper::new(0u64));
        b.iter(|| {
            let handles: Vec<_> = (0..4).map(|_| {
                let mutex = Arc::clone(&mutex);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let mut guard = mutex.lock();
                        *guard += 1;
                    }
                })
            }).collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
    
    // 原子操作
    group.bench_function("atomic_counter", |b| {
        let counter = Arc::new(AtomicCounter::new(0));
        b.iter(|| {
            let handles: Vec<_> = (0..4).map(|_| {
                let counter = Arc::clone(&counter);
                thread::spawn(move || {
                    for _ in 0..100 {
                        counter.increment();
                    }
                })
            }).collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
    
    // 自旋锁
    group.bench_function("spin_lock", |b| {
        let lock = Arc::new(SpinLock::new(0u64));
        b.iter(|| {
            let handles: Vec<_> = (0..4).map(|_| {
                let lock = Arc::clone(&lock);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let mut guard = lock.lock();
                        *guard += 1;
                    }
                })
            }).collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
    
    group.finish();
}

fn bench_rwlock_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("rwlock_performance");
    
    let rwlock = Arc::new(StdRwLockWrapper::new(vec![0u64; 1000]));
    
    // 读操作基准测试
    group.bench_function("read_heavy", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..8).map(|_| {
                let rwlock = Arc::clone(&rwlock);
                thread::spawn(move || {
                    for _ in 0..50 {
                        if let Ok(data) = rwlock.read() {
                            let _sum: u64 = data.iter().sum();
                        }
                    }
                })
            }).collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
    
    // 写操作基准测试
    group.bench_function("write_heavy", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..2).map(|_| {
                let rwlock = Arc::clone(&rwlock);
                thread::spawn(move || {
                    for _ in 0..10 {
                        if let Ok(mut data) = rwlock.write() {
                            for item in data.iter_mut() {
                                *item += 1;
                            }
                        }
                    }
                })
            }).collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
    
    group.finish();
}

fn bench_atomic_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("atomic_operations");
    
    let counter = AtomicCounter::new(0);
    
    group.bench_function("increment", |b| {
        b.iter(|| {
            counter.increment()
        })
    });
    
    group.bench_function("get", |b| {
        b.iter(|| {
            counter.get()
        })
    });
    
    group.bench_function("set", |b| {
        b.iter(|| {
            counter.set(black_box(42))
        })
    });
    
    group.bench_function("compare_and_swap", |b| {
        b.iter(|| {
            let current = counter.get();
            counter.compare_and_swap(current, current + 1)
        })
    });
    
    group.finish();
}

fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");
    
    let mut mem = MmapSharedMemory::new(1024 * 1024).unwrap(); // 1MB
    
    // 顺序写入
    group.bench_function("sequential_write", |b| {
        let data = vec![42u8; 1024];
        b.iter(|| {
            for i in 0..1000 {
                mem.write(black_box(i * 1024), black_box(&data)).unwrap();
            }
        })
    });
    
    // 随机写入
    group.bench_function("random_write", |b| {
        let data = vec![42u8; 1024];
        let offsets: Vec<usize> = (0..1000).map(|i| (i * 1024) % (1024 * 1000)).collect();
        b.iter(|| {
            for &offset in &offsets {
                mem.write(black_box(offset), black_box(&data)).unwrap();
            }
        })
    });
    
    // 顺序读取
    group.bench_function("sequential_read", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let _result = mem.read(black_box(i * 1024), black_box(1024));
            }
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_shared_memory_creation,
    bench_shared_memory_write,
    bench_shared_memory_read,
    bench_mutex_contention,
    bench_rwlock_performance,
    bench_atomic_operations,
    bench_memory_patterns
);

criterion_main!(benches);