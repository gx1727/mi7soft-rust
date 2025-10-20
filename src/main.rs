

use mi7soft_rust::common::{Message, MessageType};
use mi7soft_rust::ipc::{IpcError, IpcMutex, IpcMutexImpl, RingBuffer, SharedMemory, SharedMemoryImpl};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

// 生成唯一消息ID（简化版）
static mut MSG_ID: u64 = 0;
fn generate_msg_id() -> u64 {
    unsafe {
        MSG_ID += 1;
        MSG_ID
    }
}

// Daemon 进程：创建共享内存和锁，启动 entry 和 worker
fn run_daemon() -> Result<(), IpcError> {
    println!("[Daemon] 启动，创建共享内存和锁...");

    // 初始化共享内存和锁（确保先于 entry/worker 创建）
    let mut shm = SharedMemoryImpl::new()?;
    let _mutex = IpcMutexImpl::new()?;
    
    // 确保缓冲区正确初始化
    unsafe {
        std::ptr::write(shm.get_buffer_mut() as *mut _, RingBuffer::new());
    }

    println!("[Daemon] 启动 entry 进程...");
    let mut entry_handle = Command::new(std::env::current_exe().unwrap())
        .arg("entry")
        .stdout(Stdio::inherit())
        .spawn()
        .map_err(|e| IpcError::MemoryError(format!("启动 entry 失败: {}", e)))?;

    println!("[Daemon] 启动 worker 进程...");
    let mut worker_handle = Command::new(std::env::current_exe().unwrap())
        .arg("worker")
        .stdout(Stdio::inherit())
        .spawn()
        .map_err(|e| IpcError::MemoryError(format!("启动 worker 失败: {}", e)))?;

    // 等待子进程完成
    entry_handle.wait().unwrap();
    worker_handle.wait().unwrap();
    Ok(())
}

// Entry 进程：发送请求消息到共享内存
fn run_entry() -> Result<(), IpcError> {
    println!("[Entry] 启动，连接共享内存...");

    // 连接共享内存和锁
    let mut shm = SharedMemoryImpl::new()?;
    let mutex = IpcMutexImpl::new()?;

    // 发送3条请求消息
    for i in 1..=3 {
        let msg = Message::new(
            generate_msg_id(),
            MessageType::Request,
            &format!("请求 {}: 计算 1+{}", i, i),
        );

        // 加锁 -> 写入消息 -> 解锁
        mutex.lock().map_err(IpcError::LockError)?;
        match shm.get_buffer_mut().push(msg) {
            Ok(_) => println!("[Entry] 发送消息: {}", msg),
            Err(e) => eprintln!("[Entry] 发送失败: {}", e),
        }
        mutex.unlock().map_err(IpcError::LockError)?;

        thread::sleep(Duration::from_secs(1));
    }

    // 等待 worker 处理完成
    thread::sleep(Duration::from_secs(3));
    Ok(())
}

// Worker 进程：从共享内存读取请求并处理
fn run_worker() -> Result<(), IpcError> {
    println!("[Worker] 启动，连接共享内存...");

    // 连接共享内存和锁
    let mut shm = SharedMemoryImpl::new()?;
    let mutex = IpcMutexImpl::new()?;

    // 循环读取并处理请求
    let mut attempts = 0;
    loop {
        attempts += 1;
        // 加锁 -> 读取消息 -> 解锁
        mutex.lock().map_err(IpcError::LockError)?;
        let msg = match shm.get_buffer_mut().pop() {
            Ok(m) => m,
            Err(e) => {
                // 缓冲区为空，解锁后等待
                mutex.unlock().map_err(IpcError::LockError)?;
                println!("[Worker] 尝试 {} - 缓冲区为空: {}", attempts, e);
                if attempts > 20 {
                    println!("[Worker] 超过最大尝试次数，退出");
                    break;
                }
                thread::sleep(Duration::from_millis(500));
                continue;
            }
        };
        mutex.unlock().map_err(IpcError::LockError)?;

        // 处理请求（示例：计算 1 + i）
        if let MessageType::Request = msg.msg_type {
            let data_str = msg.get_data();
            let i: u32 = data_str.split('+').last().unwrap().trim().parse().unwrap();
            let result = 1 + i;
            let resp = Message::new(
                msg.id,
                MessageType::Response,
                &format!("结果: 1+{}={}", i, result),
            );
            println!("[Worker] 处理请求: {} -> 响应: {}", msg, resp);

            // 发送响应（实际场景可写入另一个共享内存缓冲区）
            mutex.lock().map_err(IpcError::LockError)?;
            if let Err(e) = shm.get_buffer_mut().push(resp) {
                eprintln!("[Worker] 发送响应失败: {}", e);
            }
            mutex.unlock().map_err(IpcError::LockError)?;
        }

        // 处理完3条请求后退出
        if msg.id >= 3 {
            break;
        }
    }

    Ok(())
}

fn main() -> Result<(), IpcError> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("entry") => run_entry(),
        Some("worker") => run_worker(),
        _ => run_daemon(), // 默认启动 daemon
    }
}