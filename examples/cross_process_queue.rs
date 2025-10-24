use mi7::{DefaultCrossProcessQueue, Message};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

/// 使用 CrossProcessQueue 的 tokio 异步示例
///
/// 这个示例展示了如何使用更高级的 CrossProcessQueue API
/// 来实现异步的生产者-消费者模式

/// 生产者协程：使用 DefaultCrossProcessQueue 发送消息
async fn producer_task(
    queue: Arc<DefaultCrossProcessQueue>,
    mut rx: mpsc::Receiver<Message>,
    tx_result: mpsc::Sender<Result<(), String>>,
) {
    while let Some(message) = rx.recv().await {
        // 使用 CrossProcessQueue 的 send 方法
        let result = queue.send(message.clone()).map_err(|e| e.to_string());

        match result {
            Ok(()) => {
                println!("✅ 成功发送消息 ID: {}", message.id);
                let _ = tx_result.send(Ok(())).await;
            }
            Err(error_msg) => {
                println!("❌ 发送失败: {}", error_msg);
                let _ = tx_result.send(Err(error_msg)).await;

                // 队列满时等待一段时间后重试
                sleep(Duration::from_millis(10)).await;
            }
        }
    }
}

/// 消费者协程：使用 DefaultCrossProcessQueue 接收消息
async fn consumer_task(queue: Arc<DefaultCrossProcessQueue>, tx_message: mpsc::Sender<Message>) {
    loop {
        // 使用非阻塞的 try_receive 方法
        let result = queue.try_receive().map_err(|e| e.to_string());
        match result {
            Ok(Some(message)) => {
                println!(
                    "消费者收到消息: id={}, data_len={}, timestamp={}",
                    message.id,
                    message.data.len(),
                    message.timestamp
                );

                // 发送到应用协程
                if let Err(_) = tx_message.send(message).await {
                    println!("发送到应用协程失败");
                    break;
                }
            }
            Ok(None) => {
                // 队列空时等待一段时间后重试
                sleep(Duration::from_millis(5)).await;
            }
            Err(error_msg) => {
                println!("接收消息出错: {}", error_msg);
                sleep(Duration::from_millis(10)).await;
            }
        }
    }
}

/// 应用层协程：处理业务逻辑
async fn application_task(
    tx_producer: mpsc::Sender<Message>,
    mut rx_consumer: mpsc::Receiver<Message>,
    mut rx_result: mpsc::Receiver<Result<(), String>>,
) {
    // 发送一些测试消息
    for i in 1..=10 {
        let message = Message::new(format!("测试消息 {}", i));

        if let Err(e) = tx_producer.send(message).await {
            println!("发送消息失败: {}", e);
            break;
        }

        // 等待发送结果
        if let Some(result) = rx_result.recv().await {
            match result {
                Ok(()) => println!("消息 {} 发送成功", i),
                Err(e) => println!("消息 {} 发送失败: {}", i, e),
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    // 接收处理过的消息
    let mut received_count = 0;
    while received_count < 10 {
        if let Some(message) = rx_consumer.recv().await {
            let content = String::from_utf8_lossy(&message.data);
            println!("应用层处理消息: id={}, content={}", message.id, content);
            received_count += 1;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动 CrossProcessQueue + tokio 集成示例");

    // 创建跨进程队列（使用默认配置）
    let queue = Arc::new(DefaultCrossProcessQueue::create_default(
        "/cross_process_test_queue",
    )?);

    // 打印队列配置和状态
    let config = queue.config();
    let status = queue.status();
    println!(
        "📊 队列配置: 容量={}, 槽位大小={}字节",
        config.capacity, config.slot_size
    );
    println!(
        "📊 队列状态: 容量={}, 消息数={}",
        status.capacity, status.message_count
    );

    // 创建通道
    let (tx_producer, rx_producer) = mpsc::channel::<Message>(32);
    let (tx_result, rx_result) = mpsc::channel::<Result<(), String>>(32);
    let (tx_consumer, rx_consumer) = mpsc::channel::<Message>(32);

    // 启动协程
    let producer_handle = tokio::spawn(producer_task(queue.clone(), rx_producer, tx_result));

    let consumer_handle = tokio::spawn(consumer_task(queue.clone(), tx_consumer));

    let app_handle = tokio::spawn(application_task(tx_producer, rx_consumer, rx_result));

    // 等待应用层任务完成
    let _ = app_handle.await?;

    println!("✅ 示例执行完成");

    // 清理资源
    producer_handle.abort();
    consumer_handle.abort();

    Ok(())
}

/// 多进程示例：演示如何在不同进程间使用队列
#[allow(dead_code)]
async fn multi_process_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 多进程示例");

    // 进程A：创建队列并发送消息
    let queue_sender = Arc::new(DefaultCrossProcessQueue::create("/multi_process_queue")?);

    // 进程B：连接到现有队列并接收消息
    let queue_receiver = Arc::new(DefaultCrossProcessQueue::connect("/multi_process_queue")?);

    // 在实际应用中，这两个队列会在不同的进程中使用
    println!("📡 队列创建完成，可以在不同进程间通信");

    // 发送测试消息
    for i in 1..=5 {
        let message = Message::new(format!("跨进程消息 {}", i));
        queue_sender.send(message)?;
        println!("📤 发送跨进程消息 {}", i);
    }

    // 接收消息
    for i in 1..=5 {
        match queue_receiver.try_receive()? {
            Some(message) => {
                let content = String::from_utf8_lossy(&message.data);
                println!("📥 接收跨进程消息 {}: {}", i, content);
            }
            None => {
                println!("⏳ 等待消息 {}", i);
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    Ok(())
}
