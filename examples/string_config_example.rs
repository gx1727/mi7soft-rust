use mi7::pipe::{PipeFactory, PipeConfig, PipeType};
use mi7::Message;
use std::collections::HashMap;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 字符串配置示例 ===\n");

    // 模拟从配置文件读取的配置
    let config_from_file = HashMap::from([
        ("pipe_type", "small"),
        ("pipe_name", "config_test_pipe"),
        ("backup_type", "large"),
        ("invalid_type", "unknown"),
    ]);

    // 1. 使用 PipeFactory 直接从字符串创建管道
    println!("1. 使用 PipeFactory 从字符串创建管道:");
    
    let pipe_type_str = config_from_file.get("pipe_type").unwrap();
    println!("   从配置读取的类型: '{}'", pipe_type_str);
    
    match PipeFactory::create_pipe_from_str(pipe_type_str, "test_pipe_1") {
        Ok(pipe) => {
            println!("   ✅ 成功创建管道: 容量={}, 槽大小={}", 
                    pipe.capacity(), pipe.slot_size());
        }
        Err(e) => {
            println!("   ❌ 创建失败: {}", e);
        }
    }

    // 2. 使用 PipeType::from_str 转换
    println!("\n2. 使用 PipeType::from_str 转换:");
    
    match PipeType::from_str(pipe_type_str) {
        Ok(pipe_type) => {
            println!("   ✅ 转换成功: {}", pipe_type);
            let pipe = PipeFactory::create_pipe(pipe_type, "test_pipe_2")?;
            println!("   管道信息: 容量={}, 槽大小={}", 
                    pipe.capacity(), pipe.slot_size());
        }
        Err(e) => {
            println!("   ❌ 转换失败: {}", e);
        }
    }

    // 3. 使用 PipeConfig::from_str 创建配置
    println!("\n3. 使用 PipeConfig::from_str 创建配置:");
    
    match PipeConfig::from_str(pipe_type_str) {
        Ok(config) => {
            println!("   ✅ 配置创建成功: 容量={}, 槽大小={}", 
                    config.capacity, config.slot_size);
            let pipe = config.create_pipe("test_pipe_3")?;
            println!("   管道创建成功: 容量={}, 槽大小={}", 
                    pipe.capacity(), pipe.slot_size());
        }
        Err(e) => {
            println!("   ❌ 配置创建失败: {}", e);
        }
    }

    // 4. 处理多种配置类型
    println!("\n4. 处理多种配置类型:");
    
    let types_to_test = vec!["small", "default", "large"];
    for (i, type_str) in types_to_test.iter().enumerate() {
        match PipeFactory::create_pipe_from_str(type_str, &format!("multi_test_{}", i)) {
            Ok(pipe) => {
                println!("   {} -> 容量={}, 槽大小={}, 内存={}KB", 
                        type_str, 
                        pipe.capacity(), 
                        pipe.slot_size(),
                        (pipe.capacity() * pipe.slot_size()) / 1024);
            }
            Err(e) => {
                println!("   {} -> ❌ {}", type_str, e);
            }
        }
    }

    // 5. 错误处理示例
    println!("\n5. 错误处理示例:");
    
    let invalid_type = config_from_file.get("invalid_type").unwrap();
    println!("   测试无效类型: '{}'", invalid_type);
    
    match PipeType::from_str(invalid_type) {
        Ok(_) => println!("   意外成功"),
        Err(e) => println!("   ✅ 正确处理错误: {}", e),
    }

    // 6. 支持的类型列表
    println!("\n6. 支持的类型列表:");
    println!("   {:?}", PipeType::supported_types());

    // 7. 实际使用示例：配置驱动的管道创建
    println!("\n7. 配置驱动的管道创建和使用:");
    
    let pipe_type_str = config_from_file.get("pipe_type").unwrap();
    let pipe = PipeFactory::create_pipe_from_str(pipe_type_str, "config_driven_pipe")?;
    
    // 使用管道发送和接收消息
    use mi7::shared_slot::SlotState;
    
    // 获取槽位并发送消息
    let slot_index = pipe.hold()?;
    pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;
    
    let message = Message::new(1, format!("使用{}类型管道的消息", pipe_type_str));
    let msg_id = pipe.send(slot_index, message)?;
    println!("   发送消息成功，ID: {}", msg_id);
    
    // 接收消息
    let receive_index = pipe.fetch()?;
    pipe.set_slot_state(receive_index, SlotState::INPROGRESS)?;
    let received_msg = pipe.receive(receive_index)?;
    println!("   接收消息成功，数据长度: {} 字节", received_msg.data.len());

    // 8. 配置文件模拟
    println!("\n8. 模拟配置文件场景:");
    
    // 模拟从不同来源读取配置
    let config_sources = vec![
        ("环境变量", "PIPE_TYPE=large"),
        ("JSON配置", r#"{"pipe_type": "default"}"#),
        ("TOML配置", "pipe_type = \"small\""),
    ];
    
    for (source, config_str) in config_sources {
        // 简单解析（实际应用中会使用专门的解析库）
        let extracted_type = if config_str.contains("large") {
            "large"
        } else if config_str.contains("default") {
            "default"
        } else if config_str.contains("small") {
            "small"
        } else {
            "unknown"
        };
        
        println!("   {} -> 提取类型: '{}'", source, extracted_type);
        match PipeType::from_str(extracted_type) {
            Ok(pipe_type) => {
                println!("     ✅ 有效配置: {}", pipe_type);
                let pipe = PipeFactory::create_pipe(pipe_type, &format!("{}_pipe", source.replace(" ", "_").to_lowercase()))?;
                println!("       管道创建成功: 容量={}, 槽大小={}", pipe.capacity(), pipe.slot_size());
            }
            Err(e) => println!("     ❌ 无效配置: {}", e),
        }
    }

    println!("\n=== 字符串配置示例完成 ===");
    Ok(())
}