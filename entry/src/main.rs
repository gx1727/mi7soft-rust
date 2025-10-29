mod protocols;
mod scheduler;

use mi7::{CrossProcessPipe, Message, config, logging::init_default_logging};

use protocols::{http_server, mqtt_server, tcp_server, udp_server, ws_server};
use scheduler::Scheduler;
use std::net::SocketAddr;
use std::sync::Arc;

use tracing::{debug, error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置系统
    config::init_config()?;

    // 初始化日志系统
    init_default_logging("entry")?;

    info!("启动消息生产者 (Entry)");

    // 使用配置中的队列名称
    let pipe_name = config::string("shared_memory", "name");
    let queue = Arc::new(CrossProcessPipe::<100, 4096>::connect(&pipe_name)?);
    info!("已连接到消息队列: {}", pipe_name);

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
        let bind_address = config::string("http", "bind_address");
        let port = config::string("http", "port");
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

    #[cfg(feature = "mqtt")]
    let _ = mqtt_handle.unwrap();

    Ok(())
}
