use mi7::Message;
use mi7::pipe::{PipeConfig, PipeFactory, PipeType};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 动态管道工厂示例 ===\n");

    // 1. 使用PipeType创建不同类型的管道
    println!("1. 使用PipeType创建管道:");

    // 创建小型管道
    let small_pipe = PipeFactory::create_pipe(PipeType::Small, "test_small_dynamic")?;
    println!(
        "   小型管道: 容量={}, 槽大小={}",
        small_pipe.capacity(),
        small_pipe.slot_size()
    );

    // 创建默认管道
    let default_pipe = PipeFactory::create_pipe(PipeType::Default, "test_default_dynamic")?;
    println!(
        "   默认管道: 容量={}, 槽大小={}",
        default_pipe.capacity(),
        default_pipe.slot_size()
    );

    // 创建大型管道
    let large_pipe = PipeFactory::create_pipe(PipeType::Large, "test_large_dynamic")?;
    println!(
        "   大型管道: 容量={}, 槽大小={}",
        large_pipe.capacity(),
        large_pipe.slot_size()
    );

    // 创建自定义管道
    let custom_pipe = PipeFactory::create_pipe(PipeType::Custom(50, 2048), "test_custom_dynamic")?;
    println!(
        "   自定义管道: 容量={}, 槽大小={}",
        custom_pipe.capacity(),
        custom_pipe.slot_size()
    );

    println!();

    // 2. 使用PipeConfig创建管道
    println!("2. 使用PipeConfig创建管道:");

    let config1 = PipeConfig::small();
    println!("   小型配置: {}", config1.config_type());
    println!("   总内存: {} 字节", config1.total_memory());
    println!("   是否预定义: {}", config1.is_predefined());

    let config2 = PipeConfig::new(200, 1024);
    println!("   自定义配置: {}", config2.config_type());
    println!("   总内存: {} 字节", config2.total_memory());
    println!("   是否预定义: {}", config2.is_predefined());

    // 验证配置
    match config2.validate() {
        Ok(_) => println!("   配置验证: 通过"),
        Err(e) => println!("   配置验证: 失败 - {}", e),
    }

    // 性能建议
    let suggestions = config2.performance_suggestions();
    if !suggestions.is_empty() {
        println!("   性能建议:");
        for suggestion in suggestions {
            println!("     - {}", suggestion);
        }
    }

    println!();

    // 3. 使用PipeConfig直接创建管道
    println!("3. 使用PipeConfig直接创建管道:");

    let config_pipe = config1.create_pipe("test_config_direct")?;
    println!(
        "   通过配置创建的管道: 容量={}, 槽大小={}",
        config_pipe.capacity(),
        config_pipe.slot_size()
    );

    println!();

    // 4. 配置兼容性测试
    println!("4. 配置兼容性测试:");

    let config_a = PipeConfig::small();
    let config_b = PipeConfig::small();
    let config_c = PipeConfig::default();

    println!(
        "   小型配置 vs 小型配置: {}",
        config_a.is_compatible(&config_b)
    );
    println!(
        "   小型配置 vs 默认配置: {}",
        config_a.is_compatible(&config_c)
    );

    println!();

    // 5. 动态管道操作示例
    println!("5. 动态管道操作示例:");

    // 使用动态管道进行消息传递
    let dynamic_pipe = PipeFactory::create_pipe(PipeType::Small, "test_dynamic_ops")?;

    // 获取槽位
    let slot_index = dynamic_pipe.hold()?;
    println!("   获取槽位: {}", slot_index);

    // 设置槽位状态为INPROGRESS以便发送消息
    use mi7::shared_slot::SlotState;
    dynamic_pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;
    println!("   设置槽位状态为INPROGRESS");

    // 发送消息
    let message = Message::new(1, "测试动态管道消息".to_string());

    let msg_id = dynamic_pipe.send(slot_index, message)?;
    println!("   发送消息ID: {}", msg_id);

    // 获取接收槽位
    let receive_index = dynamic_pipe.fetch()?;
    println!("   获取接收槽位: {}", receive_index);

    // 设置接收槽位状态为INPROGRESS以便接收消息
    dynamic_pipe.set_slot_state(receive_index, SlotState::INPROGRESS)?;

    // 接收消息
    let received_msg = dynamic_pipe.receive(receive_index)?;
    println!(
        "   接收消息flag: {}, 数据长度: {}",
        received_msg.flag,
        received_msg.data.len()
    );

    println!();

    // 6. 错误处理示例
    println!("6. 错误处理示例:");

    // 无效配置
    let invalid_config = PipeConfig::new(0, 1024);
    match invalid_config.validate() {
        Ok(_) => println!("   无效配置验证: 意外通过"),
        Err(e) => println!("   无效配置验证: 正确失败 - {}", e),
    }

    // 不支持的自定义配置
    match PipeFactory::create_pipe(PipeType::Custom(999, 999), "test_unsupported") {
        Ok(_) => println!("   不支持的配置: 意外成功"),
        Err(e) => println!("   不支持的配置: 正确失败 - {}", e),
    }

    println!();

    // 7. 性能对比测试
    println!("7. 性能对比测试:");

    let configs = vec![
        ("小型", PipeConfig::small()),
        ("默认", PipeConfig::default()),
        ("大型", PipeConfig::large()),
    ];

    for (name, config) in configs {
        let start = Instant::now();
        let pipe = config.create_pipe(&format!("perf_test_{}", name))?;
        let creation_time = start.elapsed();

        println!("   {} 管道创建时间: {:?}", name, creation_time);
        println!("   {} 管道内存使用: {} 字节", name, config.total_memory());
    }

    println!("\n=== 动态配置工厂示例完成 ===");
    Ok(())
}
