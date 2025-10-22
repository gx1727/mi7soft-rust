mod protocols;

use mi7::{CrossProcessQueue, Message};
use std::thread;
use std::time::Duration;
use tracing::{debug, error, info};
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;
use protocols::{http_server, mqtt_server, tcp_server, udp_server, ws_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统 - 按日期分割日志文件
    let log_dir = "logs";
    std::fs::create_dir_all(log_dir)?;

    let file_appender = rolling::daily(log_dir, "entry");
    let (non_blocking, _guard) = non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false)
                .with_thread_ids(true)
                .with_thread_names(true),
        )
        .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
        .init();

    info!("启动消息生产者 (Entry)");

    // 连接到消息队列
    let queue = CrossProcessQueue::connect("task_queue")?;
    info!("已连接到消息队列: task_queue");

    let http_handle = tokio::spawn(async move {
        let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
        info!("启动 HTTP 服务器，监听地址: {}", addr);
        http_server::run(addr, queue).await.expect("http server failed");
    });

    // Wait for servers (they run forever)
    let _ = tokio::try_join!(http_handle);


    // #[cfg(feature = "mqtt")]
    // let _ = mqtt_handle.unwrap();

    // run()?;

    Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到消息队列
    let queue = CrossProcessQueue::connect("task_queue")?;

    info!("开始发送任务消息...");

    // 发送一系列任务消息
    for i in 1..=20 {
        let message = Message::new(format!("Task {} - Process this data", i));

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
