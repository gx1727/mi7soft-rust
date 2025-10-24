use std::sync::Arc;
use tokio::signal;
use tokio::time::{sleep, Duration};
use tracing::{info, debug};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::{non_blocking, rolling};

use mi7::DefaultCrossProcessQueue;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统 - 按日期分割日志文件
    let log_dir = "logs";
    std::fs::create_dir_all(log_dir)?;
    
    let file_appender = rolling::daily(log_dir, "daemon");
    let (non_blocking, _guard) = non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false)
                .with_thread_ids(true)
                .with_thread_names(true)
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
        )
        .init();

    info!("MI7 跨进程消息队列守护进程启动");
    
    // 初始化消息队列
    let queue = Arc::new(DefaultCrossProcessQueue::create("task_queue")?);
    info!("消息队列已初始化: task_queue (容量: 100)");
    
    // 启动监控任务
    let monitor_queue: Arc<DefaultCrossProcessQueue> = Arc::clone(&queue);
    let monitor_handle = tokio::spawn(async move {
        loop {
            let status = monitor_queue.status();
            if status.message_count > 0 {
                debug!("队列状态: {}/{} 消息", status.message_count, status.capacity);
            }
            sleep(Duration::from_secs(5)).await;
        }
    });
    
    // 等待中断信号
    info!("守护进程运行中，按 Ctrl+C 停止");
    signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    
    info!("收到停止信号，正在关闭守护进程...");
    monitor_handle.abort();
    
    info!("守护进程已安全关闭");
    Ok(())
}