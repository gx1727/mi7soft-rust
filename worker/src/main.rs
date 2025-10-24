use mi7::DefaultCrossProcessQueue;
use std::env;
use std::process;
use tokio::time::{sleep, Duration};
use tracing::{info, error, debug};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::{non_blocking, rolling};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取worker ID（从命令行参数或进程ID）
    let worker_id = env::args()
        .nth(1)
        .unwrap_or_else(|| process::id().to_string());
    
    // 初始化日志系统 - 按日期分割日志文件
    let log_dir = "logs";
    std::fs::create_dir_all(log_dir)?;
    
    let file_appender = rolling::daily(log_dir, &format!("worker-{}", worker_id));
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
    
    info!("启动 Worker {} (PID: {})", worker_id, process::id());
    
    // 连接到消息队列
    let queue = DefaultCrossProcessQueue::connect("task_queue")?;
    
    info!("Worker {} 已连接到任务队列", worker_id);
    
    let mut processed_count = 0;
    let mut consecutive_empty = 0;
    
    loop {
        // 尝试接收消息
        match queue.try_receive() {
            Ok(Some(message)) => {
                consecutive_empty = 0;
                processed_count += 1;
                
                info!("Worker {} 处理任务 {}: {}", 
                         worker_id, 
                         message.id, 
                         String::from_utf8_lossy(&message.data));
                
                // 模拟任务处理时间
                let processing_time = Duration::from_millis(
                    100 + (message.id % 5) * 200  // 100-900ms的随机处理时间
                );
                sleep(processing_time).await;
                
                info!("Worker {} 完成任务 {} (耗时: {:?})", 
                         worker_id, message.id, processing_time);
                
                // 显示队列状态
                let status = queue.status();
                debug!("Worker {} 队列状态: {}/{} 消息剩余", 
                         worker_id, status.message_count, status.capacity);
            }
            Ok(None) => {
                consecutive_empty += 1;
                
                if consecutive_empty == 1 {
                    info!("Worker {} 等待新任务...", worker_id);
                }
                
                // 如果连续多次没有任务，考虑退出
                if consecutive_empty > 60 {  // 60次检查没有任务
                    info!("Worker {} 长时间无任务，准备退出", worker_id);
                    break;
                }
                
                // 短暂等待后重试
                sleep(Duration::from_millis(500)).await;
            }
            Err(e) => {
                error!("Worker {} 接收消息失败: {:?}", worker_id, e);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    info!("Worker {} 统计: 总共处理了 {} 个任务", worker_id, processed_count);
    info!("Worker {} 退出", worker_id);
    
    Ok(())
}