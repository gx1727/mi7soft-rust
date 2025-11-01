use anyhow::Result;
use async_channel::bounded;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 测试消费者修复效果");
    println!("===================");

    // 创建通道
    let (tx, rx) = bounded::<usize>(10);

    // 启动3个消费者（简化版本）
    let consumer_count = 3;
    println!("启动 {} 个消费者...", consumer_count);
    
    for i in 0..consumer_count {
        let work_rx = rx.clone();
        tokio::spawn(async move {
            println!("消费者 {} 启动，等待消息...", i);
            loop {
                match work_rx.recv().await {
                    Ok(msg) => {
                        println!("✅ 消费者 {} 接收到消息: {}", i, msg);
                        // 模拟处理时间
                        sleep(Duration::from_millis(100)).await;
                    },
                    Err(e) => {
                        println!("❌ 消费者 {} 接收消息失败: {:?}", i, e);
                        break;
                    }
                }
            }
            println!("消费者 {} 退出", i);
        });
    }

    // 等待消费者启动
    sleep(Duration::from_millis(100)).await;
    println!("所有消费者已启动\n");

    // 发送多个消息测试
    println!("📤 开始发送消息...");
    for i in 1..=6 {
        println!("发送消息 {}", i);
        tx.send(i).await?;
        sleep(Duration::from_millis(200)).await; // 间隔发送
    }

    println!("\n✅ 所有消息发送完成");
    
    // 等待消息处理完成
    sleep(Duration::from_secs(2)).await;
    
    // 关闭发送端，让消费者退出
    drop(tx);
    
    // 等待消费者退出
    sleep(Duration::from_millis(500)).await;
    
    println!("🎉 测试完成！");
    
    Ok(())
}