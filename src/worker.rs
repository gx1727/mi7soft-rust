use crate::ipc_queue::{CrossProcessQueue, Message};
use std::env;
use std::process;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取worker ID（从命令行参数或进程ID）
    let worker_id = env::args()
        .nth(1)
        .unwrap_or_else(|| process::id().to_string());
    
    println!("🔧 启动 Worker {} (PID: {})", worker_id, process::id());
    
    // 连接到消息队列
    let queue: CrossProcessQueue = CrossProcessQueue::connect("task_queue")?;
    
    println!("📡 Worker {} 已连接到任务队列", worker_id);
    
    let mut processed_count = 0;
    let mut consecutive_empty = 0;
    
    loop {
        // 使用异步方法，带超时等待
        match queue.receive_async_with_timeout(Duration::from_secs(30)).await? {
            Some(message) => {
                consecutive_empty = 0;
                processed_count += 1;
                
                println!("🔄 Worker {} 处理任务 {}: {}", 
                         worker_id, 
                         message.id, 
                         String::from_utf8_lossy(&message.data));
                
                // 模拟任务处理时间
                let processing_time = Duration::from_millis(
                    100 + (message.id % 5) * 200  // 100-900ms的随机处理时间
                );
                sleep(processing_time).await;
                
                println!("✅ Worker {} 完成任务 {} (耗时: {:?})", 
                         worker_id, message.id, processing_time);
                
                // 显示队列状态
                let status = queue.status();
                println!("📊 Worker {} 队列状态: {}/{} 消息剩余", 
                         worker_id, status.message_count, status.max_messages);
            }
            None => {
                consecutive_empty += 1;
                
                if consecutive_empty == 1 {
                    println!("⏸️  Worker {} 等待新任务...", worker_id);
                }
                
                // 如果连续多次没有任务，考虑退出
                if consecutive_empty > 2 {  // 30秒超时 * 2 = 60秒没有任务
                    println!("🏁 Worker {} 长时间无任务，准备退出", worker_id);
                    break;
                }
                
                println!("⏰ Worker {} 等待超时，继续等待...", worker_id);
            }
        }
    }
    
    println!("📈 Worker {} 统计: 总共处理了 {} 个任务", worker_id, processed_count);
    println!("👋 Worker {} 退出", worker_id);
    
    Ok(())
}