use std::sync::Arc;
use tokio::signal;
use tokio::time::{sleep, Duration};

use ipc_queue::{Result, CrossProcessQueue};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 MI7 跨进程消息队列守护进程启动");
    
    // 初始化消息队列
    let queue = Arc::new(CrossProcessQueue::create("task_queue", 100)?);
    println!("📡 消息队列已初始化: task_queue (容量: 100)");
    
    // 启动监控任务
    let monitor_queue: Arc<CrossProcessQueue> = Arc::clone(&queue);
    let monitor_handle = tokio::spawn(async move {
        loop {
            let status = monitor_queue.status();
            if status.message_count > 0 {
                println!("📊 队列状态: {}/{} 消息", status.message_count, status.capacity);
            }
            sleep(Duration::from_secs(5)).await;
        }
    });
    
    // 等待中断信号
    println!("✅ 守护进程运行中，按 Ctrl+C 停止");
    signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    
    println!("🛑 收到停止信号，正在关闭守护进程...");
    monitor_handle.abort();
    
    println!("✅ 守护进程已安全关闭");
    Ok(())
}