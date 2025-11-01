mod listener;
mod router;

use anyhow::Result;
use async_channel::bounded;
use mi7::config;
use mi7::pipe::PipeFactory;
use std::env;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化配置系统
    config::init_config()?;

    // 获取worker ID（从命令行参数或进程ID）
    let worker_id = env::args()
        .nth(1)
        .unwrap_or_else(|| process::id().to_string());

    // 使用新的通用配置读取方式获取配置信息
    let interface_name = config::string("worker", "interface_name");
    let interface_type = config::string("worker", "interface_type");
    let log_prefix = config::string("worker", "log_prefix");
    let _log_level = config::string("worker", "log_level");

    // 初始化安全的多进程日志系统 - 使用配置中的日志前缀
    mi7::logging::init_safe_multiprocess_default_logging(&log_prefix)?;

    // 创建一个生产者-多个消费者的消息队列
    let (tx, rx) = bounded::<usize>(100); // 创建一个缓冲大小为 100 的通道

    let consumer_count = 10;
    info!("启动 {} 个消费者任务...", consumer_count);

    for i in 0..consumer_count {
        let work_rx = rx.clone();
        tokio::spawn(async move {
            info!("消费者 {} 启动，等待消息...", i);
            loop {
                info!("消费者 {} 开始等待接收消息...", i);
                match work_rx.recv().await {
                    Ok(msg) => {
                        info!(
                            "消费者 {} 接收到消息: {} (时间戳: {:?})",
                            i,
                            msg,
                            std::time::SystemTime::now()
                        );
                        // 这里可以添加实际的消息处理逻辑
                        // 比如调用 router 处理消息
                    }
                    Err(e) => {
                        error!("消费者 {} 接收消息失败: {:?}", i, e);
                        break; // 通道关闭时退出循环
                    }
                }
            }
            info!("消费者 {} 退出", i);
        });
    }

    info!("所有消费者任务已启动，等待消费者准备就绪...");

    info!("启动 Worker {} (PID: {})", worker_id, process::id());

    // tx.send(1).await?;
    // info!("这时应该打印 '消费者 接收到消息: 1' ...");
    // tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    // tx.send(2).await?;
    // info!("这时应该打印 '消费者 接收到消息: 2' ...");
    // tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let pipe = match PipeFactory::connect(&interface_type, &interface_name, true) {
        Ok(pipe) => {
            info!(
                "配置信息: 队列名称={}, 槽位数={} 槽位大小={}",
                interface_name,
                pipe.capacity(),
                pipe.slot_size()
            );
            Arc::new(pipe)
        }
        Err(e) => {
            error!("连接管道失败: {:?}", e);
            return Err(e);
        }
    };

    // 启动多个消费者任务
    // let consumer_count = 2; // 假设有 3 个消费者
    // let mut consumers = vec![];
    // for i in 0..consumer_count {
    //     println!("启动消费者 {}", i);
    //
    //     let work_rx = rx.clone();
    //     // let router_handle = router::Router::new(worker_id.clone(), pipe_for_router);
    //
    //     let consumer = tokio::spawn(async move {
    //         loop {
    //             if let Ok(slot_index) = work_rx.recv().await {
    //                 println!("消费者 {} recv {}", i, slot_index);
    //             } else {
    //                 println!("消费者 {} 无消息", i);
    //             }
    //         }
    //
    //         // loop {
    //         //     println!("消费者 {} 等待收到消息", i);
    //         //     while let Ok(slot_index) = work_rx.recv().await {
    //         //         println!("消费者 {} recv {}", i, slot_index);
    //         //     }
    //         //     println!("worker {} exiting", i);
    //         // }
    //     });
    //
    //     consumers.push(consumer);
    // }

    // info!("Worker {} 已连接到任务队列: {}", worker_id, &interface_name);

    // // 克隆 pipe 用于不同的组件
    let pipe_for_listener = Arc::clone(&pipe);
    // let pipe_for_router = Arc::clone(&pipe);

    // // 创建 listener 并传递 pipe 和 worker_id
    let listener = listener::Listener::new(worker_id.clone(), pipe_for_listener, tx);

    // // 启动 listener 协程
    let listener_handle = tokio::spawn(async move {
        listener.run().await;
    });

    // // 等待 listener 协程完成
    match listener_handle.await {
        Ok(_) => {
            info!("Worker {} listener 协程正常退出", worker_id);
        }
        Err(e) => {
            error!("Worker {} listener 协程异常退出: {:?}", worker_id, e);
        }
    }

    // 等待 listener 协程完成
    // match router_handle.await {
    //     Ok(_) => {
    //         info!("Worker {} router 协程正常退出", worker_id);
    //     }
    //     Err(e) => {
    //         error!("Worker {} router 协程异常退出: {:?}", worker_id, e);
    //     }
    // }

    info!("Worker {} 主进程退出", worker_id);

    Ok(())
}
