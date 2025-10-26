mod protocols;
mod scheduler;

use mi7::{
    DefaultCrossProcessPipe, Message,
    config::{get_http_bind_address, get_http_port, get_queue_name, init_config},
    logging::init_default_logging,
};
use protocols::{http_server, mqtt_server, tcp_server, udp_server, ws_server};
use scheduler::Scheduler;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置系统
    init_config()?;

    // 初始化日志系统
    init_default_logging("entry")?;

    info!("启动消息生产者 (Entry)");

    // 使用配置中的队列名称
    let queue_name = get_queue_name();
    let queue = Arc::new(DefaultCrossProcessPipe::connect(queue_name)?);
    info!("已连接到消息队列: {}", queue_name);

    // 创建调度者
    let scheduler = Scheduler::new(queue.clone());
    let counter = scheduler.get_counter();
    let slot_sender = scheduler.get_slot_sender();

    // 启动调度者协程
    let scheduler_handle = tokio::spawn(async move {
        info!("启动调度者协程");
        scheduler.run().await;
    });

    let http_handle = tokio::spawn(async move {
        // 使用配置中的 HTTP 服务器地址和端口
        let bind_address = get_http_bind_address();
        let port = get_http_port();
        let addr: SocketAddr = format!("{}:{}", bind_address, port).parse().unwrap();
        info!("启动 HTTP 服务器，监听地址: {}", addr);
        http_server::run(addr, queue, counter, slot_sender)
            .await
            .expect("http server failed");
    });

    // 启动后台响应处理循环
    let response_handler_handle = tokio::spawn(async move {
        info!("启动后台响应处理循环");
        http_server::response_handler_loop().await;
    });

    // Wait for servers (they run forever)
    let _ = tokio::try_join!(scheduler_handle, http_handle, response_handler_handle);

    // #[cfg(feature = "mqtt")]
    // let _ = mqtt_handle.unwrap();

    // run()?;

    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到消息队列
    let queue_name = get_queue_name();
    let queue = DefaultCrossProcessPipe::connect(queue_name)?;

    info!("开始发送任务消息...");

    // 发送一系列任务消息
    for i in 1..=20 {
        let message = Message::new(0, format!("Task {} - Process this data", i));

        match queue.send(message.clone()) {
            Ok(()) => {
                info!("发送任务 {}: {}", i, String::from_utf8_lossy(&message.data));
            }
            Err(e) => {
                error!("发送任务 {} 失败: {:?}", i, e);
            }
        }

        // 显示队列状态
        let status = queue.status();
        debug!(
            "队列状态: {}/{} 消息",
            status.message_count, status.capacity
        );

        // 模拟任务生成间隔
        thread::sleep(Duration::from_millis(500));
    }

    info!("生产者完成，发送了 20 个任务");
    info!("现在可以启动多个 worker 来处理这些任务");

    // 等待 30 秒让 worker 处理任务
    info!("等待 30 秒让 worker 处理任务...");
    thread::sleep(Duration::from_secs(30));

    // 显示最终队列状态
    let final_status = queue.status();
    info!(
        "最终队列状态: {}/{} 消息",
        final_status.message_count, final_status.capacity
    );

    info!("Entry 程序结束");
    Ok(())
}
