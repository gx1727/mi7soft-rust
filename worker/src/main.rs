mod listener;

use mi7::config;
use mi7::pipe::PipeFactory;
use mi7::shared_slot::SlotState;
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
    let interface_name = config::string("worker", "interface_name");
    let interface_type = config::string("worker", "interface_type");
    let log_prefix = config::string("worker", "log_prefix");
    let log_level = config::string("worker", "log_level");

    // 初始化安全的多进程日志系统 - 使用配置中的日志前缀
    mi7::logging::init_safe_multiprocess_default_logging(&log_prefix)?;

    info!("启动 Worker {} (PID: {})", worker_id, process::id());

    let pipe = match PipeFactory::create(&interface_type, &interface_name) {
        Ok(pipe) => pipe,
        Err(e) => {
            error!("连接管道失败: {:?}", e);
            return Err(e);
        }
    };

    info!(
        "配置信息: 队列名称={}, 槽位数={} 槽位大小={}",
        interface_name,
        pipe.capacity(),
        pipe.slot_size()
    );

    // let pipe = match Arc::new(CrossProcessPipe::<100, 4096>::connect(&pipe_name)) {
    //     Ok(pipe) => {
    //         println!("✅ 成功连接到现有管道: {}", &pipe_name);
    //         pipe
    //     }
    //     Err(_) => {
    //         println!("⚠️ 连接失败，正在创建新管道: {}", &pipe_name);
    //         Arc::new(CrossProcessPipe::<100, 4096>::create(&pipe_name)
    //             .map_err(|e| format!("创建管道失败: {:?}", e))?)
    //     }
    // };

    info!("Worker {} 已连接到任务队列: {}", worker_id, &interface_name);

    let listener = listener::Listener::new(pipe);
    let handler = tokio::spawn(async move {
        listener.run().await;
    });

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
                    Ok(message) => {
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
