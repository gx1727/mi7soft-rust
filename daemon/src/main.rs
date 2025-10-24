use std::sync::Arc;
use tokio::signal;
use tokio::time::{Duration, sleep};
use tracing::{debug, info};

use mi7::{
    DefaultCrossProcessQueue,
    config::{get_queue_capacity, get_queue_name, init_config},
    logging::init_default_logging,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置系统
    init_config()?;

    // 初始化日志系统
    init_default_logging("daemon")?;

    info!("MI7 跨进程消息队列守护进程启动");

    // 使用配置中的队列名称和容量
    let queue_name = get_queue_name();
    let queue_capacity = get_queue_capacity();
    let queue = Arc::new(DefaultCrossProcessQueue::create(queue_name)?);
    info!(
        "消息队列已初始化: {} (容量: {})",
        queue_name, queue_capacity
    );

    // 启动监控任务
    let monitor_queue: Arc<DefaultCrossProcessQueue> = Arc::clone(&queue);
    let monitor_handle = tokio::spawn(async move {
        loop {
            let status = monitor_queue.status();
            if status.message_count > 0 {
                debug!(
                    "队列状态: {}/{} 消息",
                    status.message_count, status.capacity
                );
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
