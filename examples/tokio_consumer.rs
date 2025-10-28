use mi7::shared::SlotState;
use mi7::{DefaultCrossProcessPipe, Message};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Tokio 消费者进程启动");
    println!("=====================================");

    println!("🔗 开始连接到跨进程管道...");
    // 连接到跨进程管道
    let pipe = Arc::new(Mutex::new(
        DefaultCrossProcessPipe::connect_default("tokio_producer_pipe")
            .map_err(|e| format!("连接管道失败: {:?}", e))?,
    ));
    println!("✅ 跨进程管道连接成功");

    println!("🔍 正在获取管道信息...");
    let capacity = pipe.lock().await.capacity();
    let slot_size = pipe.lock().await.slot_size();
    println!(
        "✅ 管道连接成功，容量: {}, 槽位大小: {} bytes",
        capacity, slot_size
    );

    // 1. 创建 tokio::sync::mpsc 消息通道
    println!("🔧 创建消息通道...");
    let (receive_tx, receive_rx) = mpsc::channel::<usize>(100);
    let receive_rx = Arc::new(Mutex::new(receive_rx));

    println!("📡 消息通道创建完成");

    // 2. 启动监听者协程
    let listener_pipe = Arc::clone(&pipe);
    let listener_handle = tokio::spawn(async move {
        println!("👂 监听者协程启动");

        loop {
            // 监听者 pipe.fetch，获取 receive_index
            let index_option = {
                let mut pipe_guard = listener_pipe.lock().await;
                match pipe_guard.fetch() {
                    Ok(index) => Some(index),
                    Err(_) => None,
                }
                // 锁在这里自动释放
            };

            if let Some(index) = index_option {
                println!("📨 监听者获取到消息索引: {}", index);

                // 将 receive_index 发送到通道
                if receive_tx.send(index).await.is_err() {
                    eprintln!("❌ 发送接收索引失败");
                    break;
                }
            } else {
                // 没有可用消息，稍等片刻
                sleep(Duration::from_millis(10)).await;
            }
        }
    });

    // 3. 启动多个工作协程
    let worker_count = 3;
    let mut worker_handles = Vec::new();

    for worker_id in 0..worker_count {
        let worker_pipe = Arc::clone(&pipe);
        let worker_receive_rx = Arc::clone(&receive_rx);

        let worker_handle = tokio::spawn(async move {
            println!("👷 工作协程 {} 启动", worker_id);

            loop {
                // 3. 从通道获取 receive_index
                let receive_index = {
                    let mut rx_guard = worker_receive_rx.lock().await;
                    rx_guard.recv().await
                };

                if let Some(index) = receive_index {
                    println!("👷 工作协程 {} 获取到接收索引: {}", worker_id, index);

                    // 3. 检查槽位状态（快速获取锁并释放）
                    println!("🔍 工作协程 {} 开始检查槽位 {} 状态", worker_id, index);
                    {
                        let pipe_guard = worker_pipe.lock().await;
                        println!("🔍 工作协程 {} 开始检查槽位 {} 状态 222", worker_id, index);
                        match pipe_guard.get_slot_state(index) {
                            Ok(state) => {
                                println!("🔍 工作协程 {} 槽位 {} 当前状态: {:?}", worker_id, index, state);
                            }
                            Err(e) => {
                                eprintln!("❌ 工作协程 {} 获取槽位 {} 状态失败: {:?}", worker_id, index, e);
                                continue;
                            }
                        }
                        // 锁在这里自动释放
                    }

                    // 4. 设置槽位状态为 INPROGRESS（快速获取锁并释放）
                    println!("🔄 工作协程 {} 开始设置槽位 {} 状态", worker_id, index);
                    {
                        let mut pipe_guard = worker_pipe.lock().await;
                        match pipe_guard.set_slot_state(index, SlotState::INPROGRESS) {
                            Ok(_) => {
                                println!("✅ 工作协程 {} 成功设置槽位 {} 状态为 INPROGRESS", worker_id, index);
                            }
                            Err(e) => {
                                eprintln!("❌ 工作协程 {} 设置槽位 {} 状态失败: {:?}", worker_id, index, e);
                                continue;
                            }
                        }
                        // 锁在这里自动释放
                    }

                    // 5. 从slot读取数据（快速获取锁并释放）
                    println!("📥 工作协程 {} 开始接收槽位 {} 的消息", worker_id, index);
                    {
                        let mut pipe_guard = worker_pipe.lock().await;
                        match pipe_guard.receive(index) {
                            Ok(Some(received_message)) => {
                                println!(
                                    "✅ 工作协程 {} 接收到消息: {:?}",
                                    worker_id, received_message
                                );
                            }
                            Ok(None) => {
                                println!("⚠️ 工作协程 {} 槽位 {} 没有消息", worker_id, index);
                            }
                            Err(e) => {
                                eprintln!("❌ 工作协程 {} 接收消息失败: {:?}", worker_id, e);
                            }
                        }
                        // 锁在这里自动释放
                    }

                    println!("✅ 工作协程 {} 处理索引 {} 完成", worker_id, index);
                } else {
                    println!("⚠️ 工作协程 {} 通道已关闭", worker_id);
                    break;
                }

                // 控制工作频率
                sleep(Duration::from_millis(100)).await;
            }
        });

        worker_handles.push(worker_handle);
    }

    // 等待一段时间让程序运行
    println!("⏳ 程序运行中，15秒后自动退出...");
    sleep(Duration::from_secs(15)).await;

    println!("🛑 程序即将退出");

    // 取消所有任务
    listener_handle.abort();
    for handle in worker_handles {
        handle.abort();
    }

    println!("✅ 消费者进程退出");
    Ok(())
}