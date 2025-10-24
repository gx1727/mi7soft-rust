use mi7::shared_ring::SharedRingQueue;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;

// 包装器结构体，用于安全地在线程间传递共享内存指针
struct SafeSharedRing {
    ptr: *mut SharedRingQueue<1024, 256>,
}

unsafe impl Send for SafeSharedRing {}
unsafe impl Sync for SafeSharedRing {}

impl SafeSharedRing {
    fn new(name: &str) -> Self {
        let ptr = unsafe { SharedRingQueue::<1024, 256>::open(name, true) };
        Self { ptr }
    }

    unsafe fn try_push<T: bincode::Encode>(&mut self, value: &T) -> Result<u64, &'static str> {
        unsafe { (*self.ptr).try_push(value) }
    }

    unsafe fn try_pop<T: bincode::Decode<()>>(&mut self) -> Option<(u64, T)> {
        unsafe { (*self.ptr).try_pop::<T>() }
    }
}

/// 示例：使用 tokio::sync::mpsc 与 SharedRingQueue 集成
///
/// 这个示例展示了如何使用 tokio 的异步通道来协调对共享内存环形队列的访问，
/// 避免了使用条件变量的阻塞等待，实现了真正的异步操作。

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
struct Message {
    id: u64,
    content: String,
}

/// 生产者协程：负责向共享内存队列写入数据
async fn producer_task(
    queue: Arc<std::sync::Mutex<SafeSharedRing>>,
    mut rx: mpsc::Receiver<Message>,
    tx_result: mpsc::Sender<Result<u64, String>>,
) {
    while let Some(message) = rx.recv().await {
        // 尝试获取队列锁并写入数据
        let result = {
            let mut queue_guard = queue.lock().unwrap();
            unsafe { queue_guard.try_push(&message) }
        };

        match result {
            Ok(request_id) => {
                println!(
                    "✅ 成功写入消息 ID: {}, 请求 ID: {}",
                    message.id, request_id
                );
                let _ = tx_result.send(Ok(request_id)).await;
            }
            Err(e) => {
                println!("❌ 写入失败: {}", e);
                let _ = tx_result.send(Err(e.to_string())).await;

                // 队列满时等待一段时间后重试
                sleep(Duration::from_millis(10)).await;
            }
        }
    }
}

/// 消费者协程：负责从共享内存队列读取数据
async fn consumer_task(
    queue: Arc<std::sync::Mutex<SafeSharedRing>>,
    tx_message: mpsc::Sender<Message>,
) {
    loop {
        // 尝试获取队列锁并读取数据
        let result = {
            let mut queue_guard = queue.lock().unwrap();
            unsafe { queue_guard.try_pop::<Message>() }
        };

        match result {
            Some((request_id, message)) => {
                println!(
                    "消费者收到消息: request_id={}, id={}, content={}",
                    request_id, message.id, message.content
                );
                // 发送到应用协程
                if let Err(_) = tx_message.send(message).await {
                    println!("发送到应用协程失败");
                    break;
                }
            }
            None => {
                // 队列空时等待一段时间后重试
                sleep(Duration::from_millis(5)).await;
            }
        }
    }
}

/// 应用层协程：处理业务逻辑
async fn application_task(
    tx_producer: mpsc::Sender<Message>,
    mut rx_consumer: mpsc::Receiver<Message>,
    mut rx_result: mpsc::Receiver<Result<u64, String>>,
) {
    // 发送一些测试消息
    for i in 1..=10 {
        let message = Message {
            id: i,
            content: format!("测试消息 {}", i),
        };

        if let Err(e) = tx_producer.send(message).await {
            println!("发送消息失败: {}", e);
            break;
        }

        // 等待写入结果
        if let Some(result) = rx_result.recv().await {
            match result {
                Ok(request_id) => println!("消息 {} 写入成功，请求 ID: {}", i, request_id),
                Err(e) => println!("消息 {} 写入失败: {}", i, e),
            }
        }

        sleep(Duration::from_millis(100)).await;
    }

    // 接收处理过的消息
    let mut received_count = 0;
    while received_count < 10 {
        if let Some(message) = rx_consumer.recv().await {
            println!("应用层处理消息: {:?}", message);
            received_count += 1;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动 SharedRingQueue + tokio::sync::mpsc 集成示例");

    // 创建共享内存环形队列
    let queue = Arc::new(std::sync::Mutex::new(SafeSharedRing::new("/test_queue")));

    // 创建通道
    let (tx_producer, rx_producer) = mpsc::channel::<Message>(32);
    let (tx_result, rx_result) = mpsc::channel::<Result<u64, String>>(32);
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

/// 高级示例：使用多个生产者和消费者
#[allow(dead_code)]
async fn advanced_example() -> Result<(), Box<dyn std::error::Error>> {
    let queue = Arc::new(std::sync::Mutex::new(SafeSharedRing::new(
        "/advanced_queue",
    )));

    // 创建多个生产者通道
    let (_tx_main, _rx_main) = mpsc::channel::<Message>(100);
    let (tx_results, _rx_results) = mpsc::channel::<Result<u64, String>>(100);

    // 启动多个生产者协程
    let mut producer_handles = Vec::new();
    for _i in 0..3 {
        let (tx_producer, rx_producer) = mpsc::channel::<Message>(32);
        let queue_clone = queue.clone();
        let tx_results_clone = tx_results.clone();

        let handle = tokio::spawn(async move {
            producer_task(queue_clone, rx_producer, tx_results_clone).await;
        });

        producer_handles.push((handle, tx_producer));
    }

    // 启动多个消费者协程
    let mut consumer_handles = Vec::new();
    for _i in 0..2 {
        let (tx_consumer, rx_consumer) = mpsc::channel::<Message>(32);
        let queue_clone = queue.clone();

        let handle = tokio::spawn(async move {
            consumer_task(queue_clone, tx_consumer).await;
        });

        consumer_handles.push((handle, rx_consumer));
    }

    println!("🔄 高级示例：多生产者多消费者模式启动完成");

    Ok(())
}
