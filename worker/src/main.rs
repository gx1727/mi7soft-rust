use mi7::shared_slot::SlotState;
use mi7::{CrossProcessPipe, config};
use std::env;
use std::process;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置系统
    config::init_config()?;

    // 获取worker ID（从命令行参数或进程ID）
    let worker_id = env::args()
        .nth(1)
        .unwrap_or_else(|| process::id().to_string());

    // 使用新的通用配置读取方式获取配置信息
    let pipe_name = config::string("shared_memory", "name");
    let slot_size = config::int("shared_memory", "slot_size");
    let persistent = config::bool("queue", "persistent");
    let log_prefix = config::string("logging", "file_prefix");

    // 初始化安全的多进程日志系统 - 使用配置中的日志前缀
    mi7::logging::init_safe_multiprocess_default_logging(&log_prefix)?;

    info!("启动 Worker {} (PID: {})", worker_id, process::id());
    info!(
        "配置信息: 队列名称={}, 槽位大小={}, 持久化={}",
        pipe_name, slot_size, persistent
    );

    let pipe = match CrossProcessPipe::<100, 4096>::connect(&pipe_name) {
        Ok(pipe) => {
            println!("✅ 成功连接到现有管道: {}", &pipe_name);
            pipe
        }
        Err(_) => {
            println!("⚠️ 连接失败，正在创建新管道: {}", &pipe_name);
            CrossProcessPipe::<100, 4096>::create(&pipe_name)
                .map_err(|e| format!("创建管道失败: {:?}", e))?
        }
    };

    info!("Worker {} 已连接到任务队列: {}", worker_id, &pipe_name);

    let processed_count = 0;
    let mut consecutive_empty = 0;

    loop {
        // 尝试接收消息
        match pipe.fetch() {
            Ok(receive_index) => {
                println!("📥 接收到消息槽位: {}", receive_index);
                pipe.set_slot_state(receive_index, SlotState::INPROGRESS)?;

                // 成功获取到消息索引，尝试接收消息
                match pipe.receive(receive_index) {
                    Ok(Some(message)) => {
                        // 重置连续空计数
                        consecutive_empty = 0;

                        info!(
                            "Worker {} 收到任务 flag={}: {}",
                            worker_id,
                            message.flag,
                            String::from_utf8_lossy(&message.data)
                        );

                        // 模拟任务处理时间
                        let processing_time = Duration::from_millis(
                            100 + (message.timestamp % 5) * 200, // 100-900ms的随机处理时间
                        );
                        sleep(processing_time).await;

                        info!(
                            "Worker {} 完成任务 flag={} (耗时: {:?})",
                            worker_id, message.flag, processing_time
                        );

                        // 显示队列状态
                        let status = pipe.status();
                        debug!(
                            "Worker {} 队列状态: {}/{} 消息剩余",
                            worker_id, status.ready_count, status.capacity
                        );
                    }
                    Ok(None) => {
                        // 槽位为空
                        consecutive_empty += 1;
                        if consecutive_empty == 1 {
                            info!("Worker {} 等待新任务...", worker_id);
                        }
                    }
                    Err(e) => {
                        error!("Worker {} 读取消息失败: {:?}", worker_id, e);
                        consecutive_empty += 1;
                    }
                }
            }
            Err(_) => {
                // 队列为空，无法获取消息索引
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
        }
    }

    info!(
        "Worker {} 统计: 总共处理了 {} 个任务",
        worker_id, processed_count
    );
    info!("Worker {} 退出", worker_id);

    Ok(())
}
