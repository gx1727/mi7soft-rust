mod listener;
mod router;

use anyhow::Result;
use mi7::config;
use mi7::pipe::PipeFactory;
use std::env;
use std::process;
use std::sync::Arc;
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

    info!("启动 Worker {} (PID: {})", worker_id, process::id());

    let pipe = match PipeFactory::create(&interface_type, &interface_name) {
        Ok(pipe) => Arc::new(pipe),
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

    info!("Worker {} 已连接到任务队列: {}", worker_id, &interface_name);

    // 创建 listener 并传递 pipe 和 worker_id
    let listener = listener::Listener::new(worker_id.clone());

    // 克隆 pipe 用于不同的组件
    let pipe_for_listener = Arc::clone(&pipe);
    let pipe_for_router = Arc::clone(&pipe);

    // 启动 listener 协程
    let listener_handle = tokio::spawn(async move {
        listener.run(pipe_for_listener).await;
    });

    info!("Worker {} listener 协程已启动", worker_id);

    let _router = router::Router::new(worker_id.clone(), pipe_for_router);

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
