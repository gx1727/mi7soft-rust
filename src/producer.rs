use crate::ipc_queue::{CrossProcessQueue, Message};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动消息生产者 (Entry)");

    // 创建消息队列
    let queue = CrossProcessQueue::create("task_queue", 100, 1024)?;

    println!("📝 开始发送任务消息...");

    // 发送一系列任务消息
    for i in 1..=20 {
        let message = Message {
            id: i,
            data: format!("Task {} - Process this data", i).into_bytes(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        match queue.send(&message) {
            Ok(()) => {
                println!(
                    "✅ 发送任务 {}: {}",
                    i,
                    String::from_utf8_lossy(&message.data)
                );
            }
            Err(e) => {
                eprintln!("❌ 发送任务 {} 失败: {:?}", i, e);
            }
        }

        // 显示队列状态
        let status = queue.status();
        println!(
            "📊 队列状态: {}/{} 消息",
            status.message_count, status.max_messages
        );

        // 模拟任务生成间隔
        thread::sleep(Duration::from_millis(500));
    }

    println!("🏁 生产者完成，发送了 20 个任务");
    println!("💡 现在可以启动多个 worker 来处理这些任务");

    // 保持程序运行一段时间，让worker有时间处理
    println!("⏳ 等待 30 秒让 worker 处理任务...");
    thread::sleep(Duration::from_secs(30));

    // 显示最终状态
    let final_status = queue.status();
    println!(
        "📈 最终队列状态: {}/{} 消息",
        final_status.message_count, final_status.max_messages
    );

    if final_status.message_count > 0 {
        println!("⚠️  还有 {} 个未处理的任务", final_status.message_count);
    } else {
        println!("🎉 所有任务都已被处理完成！");
    }

    Ok(())
}
