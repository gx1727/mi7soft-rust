use mi7::shared_slot::SlotState;
use mi7::{CrossProcessPipe, DefaultCrossProcessPipe, Message, PipeConfig};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestMessage {
    id: u64,
    content: String,
    timestamp: u64,
}

impl TestMessage {
    fn new(id: u64, content: &str) -> Self {
        Self {
            id,
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 CrossProcessPipe 基础使用示例");
    println!("=====================================");

    // 示例1: 基本的发送和接收
    basic_send_receive_example()?;

    // 示例4: 管道状态监控
    pipe_status_example()?;

    println!("\n✅ 所有示例执行完成！");
    Ok(())
}

/// 示例1: 基本的发送和接收操作
fn basic_send_receive_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 示例1: 基本发送和接收");
    println!("------------------------");

    // 创建管道
    let pipe = DefaultCrossProcessPipe::create_default("/pipe_basic_test")?;
    println!(
        "✅ 创建管道成功，容量: {}, 槽位大小: {} bytes",
        pipe.capacity(),
        pipe.slot_size()
    );

    // 发送消息
    let message = TestMessage::new(1, "Hello from pipe!");

    // 1. 获取空槽位
    let slot_index = pipe.hold()?;
    println!("📦 获取到空槽位: {}", slot_index);

    // 2. 设置槽位状态为 INPROGRESS（这是 send 方法所期望的状态）
    pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;
    println!("🔄 设置槽位状态为 INPROGRESS");

    // 3. 发送消息到槽位
    let request_id = pipe.send(slot_index, Message::init(message.content.clone()))?;
    println!("📤 发送消息成功，请求ID: {}", request_id);

    // 4. 接收消息
    let receive_index = pipe.fetch()?;
    println!("📥 接收到消息槽位: {}", receive_index);

    // 5. 设置槽位状态为 INPROGRESS，以便 receive 方法可以读取
    pipe.set_slot_state(receive_index, SlotState::INPROGRESS)?;
    println!("🔄 设置槽位状态为 INPROGRESS");

    // 6. 释放并获取消息内容
    if let Some(received_message) = pipe.receive(receive_index)? {
        println!("✅ 接收到消息: {:?}", received_message);
    }

    Ok(())
}

/// 示例4: 管道状态监控
fn pipe_status_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 示例4: 管道状态监控");
    println!("----------------------");

    let pipe = DefaultCrossProcessPipe::create("/pipe_status_test")?;

    // 显示初始状态
    let status = pipe.status();
    println!("📈 初始状态:");
    println!("   容量: {}", status.capacity);
    println!("   槽位大小: {} bytes", status.slot_size);
    println!("   写指针: {}", status.write_pointer);
    println!("   读指针: {}", status.read_pointer);
    println!("   已使用: {}", status.used_count);
    println!("   状态统计:");
    println!("     EMPTY: {}", status.empty_count);
    println!("     WRITING: {}", status.writing_count);
    println!("     INPROGRESS: {}", status.in_progress_count);
    println!("     READING: {}", status.reading_count);
    println!("     READY: {}", status.ready_count);

    // 获取配置信息
    let config = pipe.config();
    println!(
        "⚙️  配置信息 - 容量: {}, 槽位大小: {} bytes",
        config.capacity, config.slot_size
    );

    // 发送几条消息
    for i in 0..5 {
        let slot_index = pipe.hold()?;
        let message = TestMessage::new(i, &format!("Status test message {}", i));
        pipe.send(slot_index, Message::init(message.content))?;
        println!("📤 发送消息 {}", i);
    }

    // 显示发送后状态
    let status_after_send = pipe.status();
    println!("📈 发送后状态:");
    println!("   容量: {}", status_after_send.capacity);
    println!("   槽位大小: {} bytes", status_after_send.slot_size);
    println!("   写指针: {}", status_after_send.write_pointer);
    println!("   读指针: {}", status_after_send.read_pointer);
    println!("   已使用: {}", status_after_send.used_count);
    println!("   状态统计:");
    println!("     EMPTY: {}", status_after_send.empty_count);
    println!("     PENDINGWRITE: {}", status_after_send.writing_count);
    println!("     INPROGRESS: {}", status_after_send.in_progress_count);
    println!("     PENDINGREAD: {}", status_after_send.reading_count);
    println!("     FULL: {}", status_after_send.ready_count);

    // 接收消息
    for i in 0..3 {
        let slot_index = pipe.fetch()?;
        pipe.receive(slot_index)?;
        println!("📥 接收消息 {}", i);
    }

    // 显示最终状态
    let final_status = pipe.status();
    println!("📈 最终状态:");
    println!("   容量: {}", final_status.capacity);
    println!("   槽位大小: {} bytes", final_status.slot_size);
    println!("   写指针: {}", final_status.write_pointer);
    println!("   读指针: {}", final_status.read_pointer);
    println!("   已使用: {}", final_status.used_count);
    println!("   状态统计:");
    println!("     EMPTY: {}", final_status.empty_count);
    println!("     WRITING: {}", final_status.writing_count);
    println!("     INPROGRESS: {}", final_status.in_progress_count);
    println!("     READING: {}", final_status.reading_count);
    println!("     FULL: {}", final_status.ready_count);

    Ok(())
}
