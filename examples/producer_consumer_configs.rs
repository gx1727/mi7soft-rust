use mi7::{
    pipe::{LargeCrossProcessPipe, SmallCrossProcessPipe, DefaultCrossProcessPipe},
    Message, shared_slot::SlotState,
};
use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== MI7 生产者-消费者配置示例 ===\n");

    // 示例1: 小型队列 - 轻量级消息传递
    demo_small_queue_scenario()?;
    
    // 示例2: 大型队列 - 高并发消息处理
    demo_large_queue_scenario()?;
    
    // 示例3: 配置对比测试
    demo_configuration_comparison()?;

    Ok(())
}

/// 示例1: 小型队列场景 - 系统监控消息
fn demo_small_queue_scenario() -> Result<(), Box<dyn Error>> {
    println!("🔍 示例1: 小型队列 - 系统监控消息");
    println!("=====================================");
    
    let queue_name = "system_monitor_small";
    
    // 生产者：发送系统状态消息
    let producer_pipe = SmallCrossProcessPipe::create(queue_name)?;
    println!("✅ 创建小型监控队列: {}槽位 x {}bytes = {}KB", 
             producer_pipe.capacity(), 
             producer_pipe.slot_size(),
             (producer_pipe.capacity() * producer_pipe.slot_size()) / 1024);

    // 发送几条监控消息
    let monitor_messages = vec![
        "CPU使用率: 45%",
        "内存使用率: 67%", 
        "磁盘使用率: 23%",
        "网络流量: 1.2MB/s",
    ];

    for (i, msg_content) in monitor_messages.iter().enumerate() {
        if let Ok(slot_index) = producer_pipe.hold() {
            producer_pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;
            let message = Message::new(i as u8, msg_content.to_string());
            let request_id = producer_pipe.send(slot_index, message)?;
            println!("📤 发送监控消息 {}: {} (请求ID: {})", i + 1, msg_content, request_id);
        }
    }

    // 消费者：读取监控消息
    let consumer_pipe = SmallCrossProcessPipe::connect(queue_name)?;
    println!("\n📥 开始消费监控消息:");
    
    let mut received_count = 0;
    while received_count < monitor_messages.len() {
        if let Ok(fetch_index) = consumer_pipe.fetch() {
            consumer_pipe.set_slot_state(fetch_index, SlotState::INPROGRESS)?;
            if let Ok(Some(received_message)) = consumer_pipe.receive(fetch_index) {
                let content = String::from_utf8_lossy(&received_message.data);
                println!("   ✅ 处理监控数据: {}", content);
                received_count += 1;
            }
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }

    let status = consumer_pipe.status();
    println!("📊 小型队列最终状态: 已用槽位 {}/{}", status.used_count, status.capacity);
    println!();

    Ok(())
}

/// 示例2: 大型队列场景 - 高并发数据处理
fn demo_large_queue_scenario() -> Result<(), Box<dyn Error>> {
    println!("🚀 示例2: 大型队列 - 高并发数据处理");
    println!("=====================================");
    
    let queue_name = "data_processing_large";
    
    // 生产者：发送大量数据处理任务
    let producer_pipe = LargeCrossProcessPipe::create(queue_name)?;
    println!("✅ 创建大型处理队列: {}槽位 x {}bytes = {}MB", 
             producer_pipe.capacity(), 
             producer_pipe.slot_size(),
             (producer_pipe.capacity() * producer_pipe.slot_size()) / (1024 * 1024));

    let start_time = Instant::now();
    let batch_size = 50; // 发送50条消息进行测试

    // 批量发送数据处理任务
    for i in 0..batch_size {
        if let Ok(slot_index) = producer_pipe.hold() {
            producer_pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;
            
            // 模拟较大的数据包
            let large_data = format!("数据处理任务 #{}: {}", i + 1, "x".repeat(1000));
            let message = Message::new((i % 256) as u8, large_data);
            
            let request_id = producer_pipe.send(slot_index, message)?;
            if i % 10 == 0 {
                println!("📤 批量发送进度: {}/{} (请求ID: {})", i + 1, batch_size, request_id);
            }
        }
    }

    let send_duration = start_time.elapsed();
    println!("⏱️  发送{}条消息耗时: {:?}", batch_size, send_duration);

    // 消费者：处理数据任务
    let consumer_pipe = LargeCrossProcessPipe::connect(queue_name)?;
    println!("\n📥 开始高速消费数据:");
    
    let consume_start = Instant::now();
    let mut received_count = 0;
    
    while received_count < batch_size {
        if let Ok(fetch_index) = consumer_pipe.fetch() {
            consumer_pipe.set_slot_state(fetch_index, SlotState::INPROGRESS)?;
            if let Ok(Some(received_message)) = consumer_pipe.receive(fetch_index) {
                received_count += 1;
                if received_count % 10 == 0 {
                    println!("   ✅ 处理进度: {}/{}", received_count, batch_size);
                }
                
                // 模拟数据处理时间
                thread::sleep(Duration::from_millis(1));
            }
        } else {
            thread::sleep(Duration::from_millis(1));
        }
    }

    let consume_duration = consume_start.elapsed();
    println!("⏱️  消费{}条消息耗时: {:?}", batch_size, consume_duration);

    let status = consumer_pipe.status();
    println!("📊 大型队列最终状态: 已用槽位 {}/{}", status.used_count, status.capacity);
    println!();

    Ok(())
}

/// 示例3: 配置对比测试
fn demo_configuration_comparison() -> Result<(), Box<dyn Error>> {
    println!("📈 示例3: 配置性能对比");
    println!("========================");

    // 测试参数
    let test_message_count = 20;
    let test_message = "性能测试消息".to_string();

    // 小型队列性能测试
    println!("\n🔬 小型队列性能测试:");
    let small_perf = test_queue_performance::<10, 1024>(
        "perf_test_small", 
        test_message_count, 
        &test_message
    )?;

    // 默认队列性能测试
    println!("\n🔬 默认队列性能测试:");
    let default_perf = test_queue_performance::<100, 4096>(
        "perf_test_default", 
        test_message_count, 
        &test_message
    )?;

    // 大型队列性能测试
    println!("\n🔬 大型队列性能测试:");
    let large_perf = test_queue_performance::<1000, 8192>(
        "perf_test_large", 
        test_message_count, 
        &test_message
    )?;

    // 性能对比总结
    println!("\n📊 性能对比总结:");
    println!("┌─────────────┬──────────┬──────────┬──────────────┬──────────────┐");
    println!("│ 配置类型    │ 容量     │ 槽位大小 │ 发送耗时     │ 接收耗时     │");
    println!("├─────────────┼──────────┼──────────┼──────────────┼──────────────┤");
    println!("│ 小型        │ {:8} │ {:8} │ {:12?} │ {:12?} │", 10, 1024, small_perf.0, small_perf.1);
    println!("│ 默认        │ {:8} │ {:8} │ {:12?} │ {:12?} │", 100, 4096, default_perf.0, default_perf.1);
    println!("│ 大型        │ {:8} │ {:8} │ {:12?} │ {:12?} │", 1000, 8192, large_perf.0, large_perf.1);
    println!("└─────────────┴──────────┴──────────┴──────────────┴──────────────┘");

    Ok(())
}

/// 队列性能测试函数
fn test_queue_performance<const CAPACITY: usize, const SLOT_SIZE: usize>(
    queue_name: &str,
    message_count: usize,
    test_message: &str,
) -> Result<(Duration, Duration), Box<dyn Error>> {
    use mi7::pipe::CrossProcessPipe;
    
    // 创建队列
    let producer_pipe = CrossProcessPipe::<CAPACITY, SLOT_SIZE>::create(queue_name)?;
    
    // 发送性能测试
    let send_start = Instant::now();
    for i in 0..message_count {
        if let Ok(slot_index) = producer_pipe.hold() {
            producer_pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;
            let message = Message::new(i as u8, test_message.to_string());
            producer_pipe.send(slot_index, message)?;
        }
    }
    let send_duration = send_start.elapsed();

    // 接收性能测试
    let consumer_pipe = CrossProcessPipe::<CAPACITY, SLOT_SIZE>::connect(queue_name)?;
    let receive_start = Instant::now();
    let mut received_count = 0;
    
    while received_count < message_count {
        if let Ok(fetch_index) = consumer_pipe.fetch() {
            consumer_pipe.set_slot_state(fetch_index, SlotState::INPROGRESS)?;
            if let Ok(Some(_)) = consumer_pipe.receive(fetch_index) {
                received_count += 1;
            }
        }
    }
    let receive_duration = receive_start.elapsed();

    println!("   容量: {}槽位 x {}bytes", CAPACITY, SLOT_SIZE);
    println!("   发送{}条消息耗时: {:?}", message_count, send_duration);
    println!("   接收{}条消息耗时: {:?}", message_count, receive_duration);

    Ok((send_duration, receive_duration))
}

/// 展示队列状态信息
#[allow(dead_code)]
fn display_queue_status<const CAPACITY: usize, const SLOT_SIZE: usize>(
    pipe: &mi7::pipe::CrossProcessPipe<CAPACITY, SLOT_SIZE>,
    name: &str,
) {
    let status = pipe.status();
    let config = pipe.config();
    
    println!("📋 {} 状态信息:", name);
    println!("   配置: {:?}", config);
    println!("   状态: 已用 {}/{}, READY: {}, EMPTY: {}", 
             status.used_count, status.capacity, 
             status.ready_count, status.empty_count);
    println!("   内存使用: {} KB", (status.capacity * status.slot_size) / 1024);
}