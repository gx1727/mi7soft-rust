mod listener;
mod operator;
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

    info!("启动 Worker {} (PID: {})", worker_id, process::id());

    // 创建 pipe
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
    let consumer_count = 3; // 假设有 3 个消费者
    for i in 0..consumer_count {
        let work_rx = rx.clone();
        let pipe_for_router = Arc::clone(&pipe);

        tokio::spawn(async move {
            let operator_handle = operator::Operator::new(work_rx, pipe_for_router);
            match operator_handle.run(i).await {
                Ok(_) => {
                    info!("消费者 {} 正常退出", i);
                }
                Err(e) => {
                    error!("消费者 {} 异常退出: {:?}", i, e);
                }
            }
        });
    }

    // 创建 listener 并传递 pipe 和 worker_id
    let pipe_for_listener = Arc::clone(&pipe);
    let listener = listener::Listener::new(worker_id.clone(), pipe_for_listener, tx);

    // 启动 listener 协程
    let listener_handle = tokio::spawn(async move {
        listener.run().await;
    });

    // 等待 listener 协程完成
    match listener_handle.await {
        Ok(_) => {
            info!("Worker {} listener 协程正常退出", worker_id);
        }
        Err(e) => {
            error!("Worker {} listener 协程异常退出: {:?}", worker_id, e);
        }
    }

    info!("Worker {} 主进程退出", worker_id);

    Ok(())
}
