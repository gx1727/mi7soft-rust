use mi7::{
    DefaultCrossProcessQueue,
    config::{get_queue_name, init_config},
};
use std::env;
use std::process;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置系统
    init_config()?;

    // 获取worker ID（从命令行参数或进程ID）
    let worker_id = env::args()
        .nth(1)
        .unwrap_or_else(|| process::id().to_string());

    // 初始化安全的多进程日志系统 - 所有 worker 写入同一个日志文件
    mi7::logging::init_safe_multiprocess_default_logging("workers")?;

    info!("启动 Worker {} (PID: {})", worker_id, process::id());

    // 使用配置中的队列名称连接到消息队列
    let queue_name = get_queue_name();
    let queue = DefaultCrossProcessQueue::connect(queue_name)?;

    info!("Worker {} 已连接到任务队列: {}", worker_id, queue_name);

    let mut processed_count = 0;
    let mut consecutive_empty = 0;

    loop {
        // 尝试接收消息
        match queue.try_receive() {
            Ok(Some(message)) => {
                consecutive_empty = 0;
                processed_count += 1;

                info!(
                    "Worker {} 处理任务 {}: {}",
                    worker_id,
                    message.id,
                    String::from_utf8_lossy(&message.data)
                );

                // 模拟任务处理时间
                let processing_time = Duration::from_millis(
                    100 + (message.id % 5) * 200, // 100-900ms的随机处理时间
                );
                sleep(processing_time).await;

                info!(
                    "Worker {} 完成任务 {} (耗时: {:?})",
                    worker_id, message.id, processing_time
                );

                // 显示队列状态
                let status = queue.status();
                debug!(
                    "Worker {} 队列状态: {}/{} 消息剩余",
                    worker_id, status.message_count, status.capacity
                );
            }
            Ok(None) => {
                consecutive_empty += 1;

                if consecutive_empty == 1 {
                    info!("Worker {} 等待新任务...", worker_id);
                }

                // 如果连续多次没有任务，考虑退出
                if consecutive_empty > 60 {
                    // 60次检查没有任务
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

    info!(
        "Worker {} 统计: 总共处理了 {} 个任务",
        worker_id, processed_count
    );
    info!("Worker {} 退出", worker_id);

    Ok(())
}
