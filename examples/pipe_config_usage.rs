use mi7::{
    pipe::{LargeCrossProcessPipe, PipeConfig, SmallCrossProcessPipe},
    Message,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MI7 队列配置使用示例 ===\n");

    // 方式1: 使用预定义的类型别名
    demo_with_type_aliases()?;

    // 方式2: 使用配置结构体
    demo_with_config_struct()?;

    Ok(())
}

/// 使用预定义的类型别名
fn demo_with_type_aliases() -> Result<(), Box<dyn Error>> {
    println!("1. 使用预定义类型别名:");

    // 小型队列配置：10个槽位，每个1KB
    println!("   创建小型队列 (10槽位 x 1KB)...");
    let small_pipe = SmallCrossProcessPipe::create("small_queue_demo")?;
    println!("   小型队列状态: {:?}", small_pipe.status());
    println!("   容量: {}, 槽位大小: {} bytes\n", 
             small_pipe.capacity(), small_pipe.slot_size());

    // 大型队列配置：1000个槽位，每个8KB
    println!("   创建大型队列 (1000槽位 x 8KB)...");
    let large_pipe = LargeCrossProcessPipe::create("large_queue_demo")?;
    println!("   大型队列状态: {:?}", large_pipe.status());
    println!("   容量: {}, 槽位大小: {} bytes\n", 
             large_pipe.capacity(), large_pipe.slot_size());

    // 演示基本操作
    demo_basic_operations(&small_pipe, "小型队列")?;
    demo_basic_operations(&large_pipe, "大型队列")?;

    Ok(())
}

/// 使用配置结构体
fn demo_with_config_struct() -> Result<(), Box<dyn Error>> {
    println!("2. 使用配置结构体:");

    // 获取预定义配置
    let small_config = PipeConfig::small();
    let large_config = PipeConfig::large();

    println!("   小型配置: {:?}", small_config);
    println!("   大型配置: {:?}", large_config);

    // 自定义配置
    let custom_config = PipeConfig::new(50, 2048); // 50槽位，每个2KB
    println!("   自定义配置: {:?}\n", custom_config);

    // 注意：由于泛型参数在编译时确定，配置验证主要用于运行时检查
    // 实际创建时仍需要使用对应的类型别名或泛型参数

    Ok(())
}

/// 演示基本队列操作
fn demo_basic_operations<const CAPACITY: usize, const SLOT_SIZE: usize>(
    pipe: &mi7::pipe::CrossProcessPipe<CAPACITY, SLOT_SIZE>,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    println!("   {} 基本操作演示:", name);

    // 获取空槽位
    let slot_index = pipe.hold()?;
    println!("     获取到槽位: {}", slot_index);

    // 设置槽位状态为 INPROGRESS
    pipe.set_slot_state(slot_index, mi7::shared_slot::SlotState::INPROGRESS)?;

    // 发送消息
    let message = Message::new(1, format!("来自{}的测试消息", name));

    let request_id = pipe.send(slot_index, message)?;
    println!("     发送消息，请求ID: {}", request_id);

    // 获取状态
    let status = pipe.status();
    println!("     当前状态 - 已用槽位: {}/{}, READY槽位: {}", 
             status.used_count, status.capacity, status.ready_count);

    // 接收消息
    if let Ok(fetch_index) = pipe.fetch() {
        pipe.set_slot_state(fetch_index, mi7::shared_slot::SlotState::INPROGRESS)?;
        if let Ok(Some(received_message)) = pipe.receive(fetch_index) {
            println!("     接收到消息: {:?}", received_message);
        }
    }

    println!();
    Ok(())
}

/// 性能对比演示
#[allow(dead_code)]
fn performance_comparison() -> Result<(), Box<dyn Error>> {
    println!("3. 性能对比:");

    let small_pipe = SmallCrossProcessPipe::create("perf_small")?;
    let large_pipe = LargeCrossProcessPipe::create("perf_large")?;

    println!("   小型队列内存占用: ~{} KB", 
             (small_pipe.capacity() * small_pipe.slot_size()) / 1024);
    println!("   大型队列内存占用: ~{} KB", 
             (large_pipe.capacity() * large_pipe.slot_size()) / 1024);

    // 这里可以添加更多性能测试代码...

    Ok(())
}