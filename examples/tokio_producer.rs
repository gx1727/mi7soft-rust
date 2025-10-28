use mi7::shared::SlotState;
use mi7::{DefaultCrossProcessPipe, Message};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

#[derive(Debug, Clone)]
struct WorkMessage {
    id: u64,
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Tokio 生产者进程启动");
    println!("=====================================");

    println!("🔧 开始创建跨进程管道...");

    // 先尝试连接到现有管道，如果失败则创建新管道
    let pipe_name = "tokio_producer_pipe";
    let pipe_instance = match DefaultCrossProcessPipe::create_default(pipe_name) {
        Ok(pipe) => {
            println!("✅ 成功连接到现有管道: {}", pipe_name);
            pipe
        }
        Err(_) => {
            println!("⚠️ 连接失败，正在创建新管道: {}", pipe_name);
            DefaultCrossProcessPipe::create_default(pipe_name)
                .map_err(|e| format!("创建管道失败: {:?}", e))?
        }
    };

    let pipe = Arc::new(Mutex::new(pipe_instance));
    println!("✅ 跨进程管道创建成功");

    println!("🔍 正在获取管道信息...");
    let capacity = pipe.lock().await.capacity();
    let slot_size = pipe.lock().await.slot_size();
    println!(
        "✅ 管道创建成功，容量: {}, 槽位大小: {} bytes",
        capacity, slot_size
    );

    // 1. 创建 tokio::sync::mpsc 消息通道
    println!("🔧 创建消息通道...");
    let (slot_tx, slot_rx) = mpsc::channel::<usize>(100);

    // 创建工作消息通道
    let (work_tx, work_rx) = mpsc::channel::<WorkMessage>(100);
    let work_rx = Arc::new(Mutex::new(work_rx));
    let slot_rx = Arc::new(Mutex::new(slot_rx));

    println!("📡 消息通道创建完成");

    // 2. 启动调度员协程
    let scheduler_pipe = Arc::clone(&pipe);
    let scheduler_handle = tokio::spawn(async move {
        println!("📋 调度员协程启动");

        loop {
            // 尝试获取空槽位
            let index_option = {
                let mut pipe_guard = scheduler_pipe.lock().await;
                match pipe_guard.hold() {
                    Ok(index) => Some(index),
                    Err(_) => None,
                }
            };

            if let Some(index) = index_option {
                println!("📦 调度员获取到空槽位: {}", index);

                // 将 slot_index 发送到通道
                if slot_tx.send(index).await.is_err() {
                    eprintln!("❌ 发送槽位索引失败");
                    break;
                }
            } else {
                // 没有可用槽位，稍等片刻
                sleep(Duration::from_millis(10)).await;
            }
        }
    });

    // 3. 启动多个工作协程
    let worker_count = 3;
    let mut worker_handles = Vec::new();

    for worker_id in 0..worker_count {
        let worker_pipe = Arc::clone(&pipe);
        let worker_work_rx = Arc::clone(&work_rx);
        let worker_slot_rx = Arc::clone(&slot_rx);

        let worker_handle = tokio::spawn(async move {
            println!("👷 工作协程 {} 启动", worker_id);

            loop {
                // 3. 从通道获取 slot_index
                let slot_index = {
                    let mut rx_guard = worker_slot_rx.lock().await;
                    rx_guard.recv().await
                };

                if let Some(index) = slot_index {
                    println!("👷 工作协程 {} 获取到槽位: {}", worker_id, index);

                    // 从工作消息通道获取消息
                    let work_message = {
                        let mut work_rx_guard = worker_work_rx.lock().await;
                        work_rx_guard.recv().await
                    };

                    if let Some(message) = work_message {
                        println!("👷 工作协程 {} 处理消息: {:?}", worker_id, message);

                        // 4. 设置槽位状态为 INPROGRESS
                        {
                            let mut pipe_guard = worker_pipe.lock().await;
                            if pipe_guard
                                .set_slot_state(index, SlotState::INPROGRESS)
                                .is_err()
                            {
                                eprintln!("❌ 工作协程 {} 设置槽位状态失败", worker_id);
                                continue;
                            }
                        }

                        println!(
                            "🔄 工作协程 {} 设置槽位 {} 状态为 INPROGRESS",
                            worker_id, index
                        );

                        // 5. 发送消息到槽位
                        let message_content = message.content.clone();
                        {
                            let mut pipe_guard = worker_pipe.lock().await;
                            match pipe_guard.send(index, Message::init(message_content)) {
                                Ok(id) => {
                                    println!(
                                        "✅ 工作协程 {} 发送消息成功，请求ID: {}",
                                        worker_id, id
                                    );

                                    // 6. 设置槽位状态为 READY，让消费者可以获取
                                    if let Err(e) =
                                        pipe_guard.set_slot_state(index, SlotState::READY)
                                    {
                                        eprintln!(
                                            "❌ 工作协程 {} 设置槽位 {} 为 READY 失败: {:?}",
                                            worker_id, index, e
                                        );
                                    } else {
                                        println!(
                                            "🔄 工作协程 {} 设置槽位 {} 状态为 READY",
                                            worker_id, index
                                        );
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ 工作协程 {} 发送消息失败: {:?}", worker_id, e);
                                }
                            }
                        }
                    } else {
                        println!("⚠️ 工作协程 {} 没有获取到工作消息", worker_id);
                    }
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

    // 生成测试消息
    let message_producer = tokio::spawn(async move {
        println!("📝 消息生成器启动");

        for i in 0..20 {
            let message = WorkMessage {
                id: i,
                content: format!("Hello from producer, message {}", i),
            };

            if work_tx.send(message.clone()).await.is_err() {
                eprintln!("❌ 发送工作消息失败");
                break;
            }

            println!("📤 生成消息: {}", i);
            sleep(Duration::from_millis(200)).await;
        }

        println!("📝 消息生成完成");
    });

    // 等待一段时间让程序运行
    println!("⏳ 程序运行中，10秒后自动退出...");
    sleep(Duration::from_secs(10)).await;

    println!("🛑 程序即将退出");

    // 等待消息生成器完成
    let _ = message_producer.await;

    // 取消所有任务
    scheduler_handle.abort();
    for handle in worker_handles {
        handle.abort();
    }

    println!("✅ 生产者进程退出");
    Ok(())
}
