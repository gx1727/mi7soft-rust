use mi7::pipe::{PipeFactory, PipeConfig, PipeType};
use mi7::Message;
use std::str::FromStr;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 配置文件读取示例 ===\n");

    // 1. 读取配置文件
    println!("1. 读取配置文件:");
    let config_content = fs::read_to_string("pipe_config.toml")?;
    println!("   配置文件内容:");
    println!("{}", config_content);

    // 2. 简单解析配置（实际项目中建议使用 toml 或 serde 库）
    println!("\n2. 解析配置:");
    
    let pipe_type_str = extract_config_value(&config_content, "type")?;
    let pipe_name = extract_config_value(&config_content, "name")?;
    
    println!("   提取的管道类型: '{}'", pipe_type_str);
    println!("   提取的管道名称: '{}'", pipe_name);

    // 3. 使用配置创建管道
    println!("\n3. 使用配置创建管道:");
    
    let pipe = PipeFactory::create_pipe_from_str(&pipe_type_str, &pipe_name)?;
    println!("   ✅ 管道创建成功:");
    println!("     类型: {}", PipeType::from_str(&pipe_type_str)?);
    println!("     名称: {}", pipe_name);
    println!("     容量: {}", pipe.capacity());
    println!("     槽大小: {} 字节", pipe.slot_size());
    println!("     总内存: {} KB", (pipe.capacity() * pipe.slot_size()) / 1024);

    // 4. 验证管道功能
    println!("\n4. 验证管道功能:");
    
    use mi7::shared_slot::SlotState;
    
    // 发送消息
    let slot_index = pipe.hold()?;
    pipe.set_slot_state(slot_index, SlotState::INPROGRESS)?;
    
    let message = Message::new(1, format!("来自{}配置的消息", pipe_type_str));
    let msg_id = pipe.send(slot_index, message)?;
    println!("   发送消息成功，ID: {}", msg_id);
    
    // 接收消息
    let receive_index = pipe.fetch()?;
    pipe.set_slot_state(receive_index, SlotState::INPROGRESS)?;
    let received_msg = pipe.receive(receive_index)?;
    println!("   接收消息成功，数据长度: {} 字节", received_msg.data.len());

    // 5. 配置验证
    println!("\n5. 配置验证:");
    
    let config = PipeConfig::from_str(&pipe_type_str)?;
    match config.validate() {
        Ok(_) => {
            println!("   ✅ 配置验证通过");
            println!("   内存使用: {} 字节", config.total_memory());
            
            if config.is_predefined() {
                println!("   这是预定义配置");
            } else {
                println!("   这是自定义配置");
            }
            
            // 显示性能建议
            let suggestions = config.performance_suggestions();
            if !suggestions.is_empty() {
                println!("   性能建议:");
                for suggestion in suggestions {
                    println!("     - {}", suggestion);
                }
            }
        }
        Err(e) => {
            println!("   ❌ 配置验证失败: {}", e);
        }
    }

    // 6. 环境变量覆盖示例
    println!("\n6. 环境变量覆盖示例:");
    
    // 模拟环境变量
    let env_pipe_type = std::env::var("PIPE_TYPE").unwrap_or(pipe_type_str.clone());
    println!("   环境变量 PIPE_TYPE: '{}'", env_pipe_type);
    
    if env_pipe_type != pipe_type_str {
        println!("   环境变量覆盖了配置文件设置");
        let env_pipe = PipeFactory::create_pipe_from_str(&env_pipe_type, "env_override_pipe")?;
        println!("   使用环境变量创建的管道: 容量={}, 槽大小={}", 
                env_pipe.capacity(), env_pipe.slot_size());
    } else {
        println!("   使用配置文件中的默认设置");
    }

    // 7. 错误处理示例
    println!("\n7. 错误处理示例:");
    
    let invalid_configs = vec!["invalid", "SMALL", "medium"];
    for invalid_config in invalid_configs {
        match PipeType::from_str(invalid_config) {
            Ok(_) => println!("   '{}' -> 意外成功", invalid_config),
            Err(e) => println!("   '{}' -> ❌ {}", invalid_config, e),
        }
    }

    println!("\n=== 配置文件读取示例完成 ===");
    Ok(())
}

/// 简单的配置值提取函数（实际项目中建议使用专门的配置库）
fn extract_config_value(content: &str, key: &str) -> Result<String, Box<dyn std::error::Error>> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(key) && line.contains('=') {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                let value = parts[1].trim().trim_matches('"');
                return Ok(value.to_string());
            }
        }
    }
    Err(format!("配置项 '{}' 未找到", key).into())
}