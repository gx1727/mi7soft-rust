use mi7::{
    Message, QueueConfig, 
    CrossProcessQueue, DefaultCrossProcessQueue, 
    SmallCrossProcessQueue, LargeCrossProcessQueue
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 配置化队列测试开始");
    
    // 测试1: 使用默认配置
    test_default_queue().await?;
    
    // 测试2: 使用小型队列配置
    test_small_queue().await?;
    
    // 测试3: 使用大型队列配置
    test_large_queue().await?;
    
    // 测试4: 使用自定义配置
    test_custom_queue().await?;
    
    // 测试5: 配置验证
    test_config_validation().await?;
    
    println!("✅ 所有配置化队列测试完成");
    Ok(())
}

/// 测试默认配置队列
async fn test_default_queue() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 测试1: 默认配置队列 (100槽位, 4KB)");
    
    let queue = DefaultCrossProcessQueue::create_default("test_default_queue")?;
    
    println!("队列配置: {:?}", queue.config());
    println!("队列容量: {}", queue.capacity());
    println!("槽位大小: {} bytes", queue.slot_size());
    
    // 发送几条消息
    for i in 0..5 {
        let message = Message::new(format!("默认队列消息 {}", i));
        queue.send(message)?;
        println!("✅ 发送消息 {}", i);
    }
    
    // 接收消息
    for i in 0..5 {
        if let Some(message) = queue.try_receive()? {
            let data = String::from_utf8_lossy(&message.data);
            println!("📨 接收消息 {}: {}", i, data);
        }
    }
    
    Ok(())
}

/// 测试小型队列配置
async fn test_small_queue() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 测试2: 小型配置队列 (10槽位, 1KB)");
    
    let queue = SmallCrossProcessQueue::create("test_small_queue")?;
    
    println!("队列配置: {:?}", queue.config());
    println!("队列容量: {}", queue.capacity());
    println!("槽位大小: {} bytes", queue.slot_size());
    
    // 测试队列容量限制
    println!("🔄 测试小队列容量限制...");
    let mut sent_count = 0;
    
    // 尝试发送超过容量的消息
    for i in 0..15 {
        let message = Message::new(format!("小队列消息 {}", i));
        match queue.send(message) {
            Ok(_) => {
                sent_count += 1;
                println!("✅ 发送消息 {} 成功", i);
            }
            Err(e) => {
                println!("❌ 发送消息 {} 失败: {}", i, e);
                break;
            }
        }
    }
    
    println!("📈 成功发送 {} 条消息", sent_count);
    
    Ok(())
}

/// 测试大型队列配置
async fn test_large_queue() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 测试3: 大型配置队列 (1000槽位, 8KB)");
    
    let queue = LargeCrossProcessQueue::create("test_large_queue")?;
    
    println!("队列配置: {:?}", queue.config());
    println!("队列容量: {}", queue.capacity());
    println!("槽位大小: {} bytes", queue.slot_size());
    
    // 测试大容量发送
    println!("🔄 测试大队列批量发送...");
    let batch_size = 100;
    
    for i in 0..batch_size {
        let large_data = format!("大队列消息 {} - {}", i, "x".repeat(1000));
        let message = Message::new(large_data);
        queue.send(message)?;
        
        if i % 20 == 0 {
            println!("✅ 已发送 {} 条消息", i + 1);
        }
    }
    
    println!("📈 成功发送 {} 条大消息", batch_size);
    
    // 批量接收
    let mut received_count = 0;
    while let Some(_message) = queue.try_receive()? {
        received_count += 1;
        if received_count % 20 == 0 {
            println!("📨 已接收 {} 条消息", received_count);
        }
    }
    
    println!("📈 成功接收 {} 条消息", received_count);
    
    Ok(())
}

/// 测试自定义配置
async fn test_custom_queue() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 测试4: 自定义配置队列 (50槽位, 2KB)");
    
    // 定义自定义配置
    type CustomQueue = CrossProcessQueue<50, 2048>;
    let queue = CustomQueue::create("test_custom_queue")?;
    
    println!("队列配置: {:?}", queue.config());
    println!("队列容量: {}", queue.capacity());
    println!("槽位大小: {} bytes", queue.slot_size());
    
    // 测试自定义配置的性能
    let start_time = std::time::Instant::now();
    
    for i in 0..30 {
        let message = Message::new(format!("自定义队列消息 {}", i));
        queue.send(message)?;
    }
    
    let send_duration = start_time.elapsed();
    println!("⏱️  发送30条消息耗时: {:?}", send_duration);
    
    let start_time = std::time::Instant::now();
    let mut count = 0;
    while let Some(_message) = queue.try_receive()? {
        count += 1;
    }
    
    let receive_duration = start_time.elapsed();
    println!("⏱️  接收{}条消息耗时: {:?}", count, receive_duration);
    
    Ok(())
}

/// 测试配置验证
async fn test_config_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 测试5: 配置验证");
    
    // 测试配置创建
    let config1 = QueueConfig::default();
    let config2 = QueueConfig::small();
    let config3 = QueueConfig::large();
    let config4 = QueueConfig::new(25, 512);
    
    println!("默认配置: {:?}", config1);
    println!("小型配置: {:?}", config2);
    println!("大型配置: {:?}", config3);
    println!("自定义配置: {:?}", config4);
    
    // 测试配置验证（这会失败，因为配置不匹配）
    println!("\n🔍 测试配置验证...");
    let wrong_config = QueueConfig::new(200, 8192);
    
    match DefaultCrossProcessQueue::create_with_config("test_validation", wrong_config) {
        Ok(_) => println!("❌ 配置验证失败：应该拒绝错误配置"),
        Err(e) => println!("✅ 配置验证成功：{}", e),
    }
    
    // 测试正确配置
    let correct_config = QueueConfig::new(100, 4096);
    match DefaultCrossProcessQueue::create_with_config("test_validation_correct", correct_config) {
        Ok(_) => println!("✅ 正确配置验证成功"),
        Err(e) => println!("❌ 正确配置验证失败：{}", e),
    }
    
    Ok(())
}