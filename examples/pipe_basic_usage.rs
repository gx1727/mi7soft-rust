use anyhow::Result;
use mi7::pipe::PipeFactory;
use mi7::shared_slot::SlotState;
use mi7::{CrossProcessPipe, Message};

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

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 CrossProcessPipe 基础使用示例");
    println!("=====================================");

    // 示例1: 基本的发送和接收
    basic_send_receive_example()?;

    println!("\n✅ 所有示例执行完成！");
    Ok(())
}

/// 示例1: 基本的发送和接收操作
fn basic_send_receive_example() -> Result<()> {
    println!("\n📝 示例1: 基本发送和接收");
    println!("------------------------");

    // 创建管道
    let pipe = PipeFactory::connect("large", "work_req_pipe", false)?;
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

    // // 4. 接收消息
    // let receive_index = pipe.fetch()?;
    // println!("📥 接收到消息槽位: {}", receive_index);
    //
    // // 5. 设置槽位状态为 INPROGRESS，以便 receive 方法可以读取
    // pipe.set_slot_state(receive_index, SlotState::INPROGRESS)?;
    // println!("🔄 设置槽位状态为 INPROGRESS");
    //
    // // 6. 释放并获取消息内容
    // let received_message = pipe.receive(receive_index)?;
    // println!("✅ 接收到消息: {:?}", received_message);

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


    Ok(())
}
