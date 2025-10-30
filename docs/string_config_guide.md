# 字符串配置使用指南

本指南介绍如何使用字符串配置来创建和管理管道，特别适用于从配置文件、环境变量或其他外部配置源读取配置的场景。

## 概述

MI7 管道系统现在支持从字符串配置创建管道，这使得配置驱动的应用程序开发变得更加简单。您可以：

- 从配置文件读取字符串类型（如 "small", "default", "large"）
- 直接使用字符串参数创建管道
- 进行配置验证和错误处理

## 支持的字符串类型

| 字符串 | 对应类型 | 容量 | 槽大小 | 总内存 |
|--------|----------|------|--------|--------|
| "small" | PipeType::Small | 10 | 1024 字节 | 10 KB |
| "default" | PipeType::Default | 100 | 4096 字节 | 400 KB |
| "large" | PipeType::Large | 1000 | 8192 字节 | 8000 KB |

注意：字符串匹配不区分大小写，"SMALL" 和 "small" 都有效。

## 使用方法

### 1. 导入必要的模块

```rust
use mi7::pipe::{PipeFactory, PipeConfig, PipeType};
use std::str::FromStr;
```

### 2. 使用 PipeFactory 从字符串创建管道

```rust
// 从字符串直接创建管道
let pipe = PipeFactory::create_pipe_from_str("small", "my_pipe")?;

// 连接到现有管道
let pipe = PipeFactory::connect_pipe_from_str("large", "existing_pipe")?;
```

### 3. 使用 PipeType::from_str 转换

```rust
// 先转换为 PipeType，再创建管道
let pipe_type = PipeType::from_str("default")?;
let pipe = PipeFactory::create_pipe(pipe_type, "my_pipe")?;
```

### 4. 使用 PipeConfig::from_str 创建配置

```rust
// 从字符串创建配置
let config = PipeConfig::from_str("small")?;
let pipe = config.create_pipe("my_pipe")?;
```

## 配置文件集成

### TOML 配置文件示例

```toml
# pipe_config.toml
[pipe]
type = "small"
name = "my_application_pipe"

[backup_pipe]
type = "large"
name = "backup_pipe"
```

### 读取配置文件

```rust
use std::fs;

fn load_pipe_from_config() -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
    // 读取配置文件
    let config_content = fs::read_to_string("pipe_config.toml")?;
    
    // 解析配置（简单示例，实际项目建议使用 toml 库）
    let pipe_type = extract_config_value(&config_content, "type")?;
    let pipe_name = extract_config_value(&config_content, "name")?;
    
    // 创建管道
    PipeFactory::create_pipe_from_str(&pipe_type, &pipe_name)
}
```

## 环境变量支持

```rust
// 支持环境变量覆盖
let pipe_type = std::env::var("PIPE_TYPE").unwrap_or("default".to_string());
let pipe = PipeFactory::create_pipe_from_str(&pipe_type, "env_pipe")?;
```

## 错误处理

```rust
match PipeType::from_str("invalid_type") {
    Ok(pipe_type) => {
        // 创建管道
        let pipe = PipeFactory::create_pipe(pipe_type, "my_pipe")?;
    }
    Err(e) => {
        eprintln!("无效的管道类型: {}", e);
        // 错误信息会显示支持的类型列表
    }
}
```

## 配置验证

```rust
let config = PipeConfig::from_str("small")?;

// 验证配置
match config.validate() {
    Ok(_) => {
        println!("配置有效");
        println!("内存使用: {} 字节", config.total_memory());
        
        // 获取性能建议
        for suggestion in config.performance_suggestions() {
            println!("建议: {}", suggestion);
        }
    }
    Err(e) => {
        eprintln!("配置无效: {}", e);
    }
}
```

## 实际应用示例

### 1. Web 应用配置

```rust
// 从应用配置创建管道
struct AppConfig {
    pipe_type: String,
    pipe_name: String,
}

impl AppConfig {
    fn create_pipe(&self) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        PipeFactory::create_pipe_from_str(&self.pipe_type, &self.pipe_name)
    }
}
```

### 2. 微服务配置

```rust
// 根据服务规模选择管道类型
fn create_service_pipe(service_scale: &str) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
    let pipe_type = match service_scale {
        "micro" => "small",
        "standard" => "default", 
        "enterprise" => "large",
        _ => "default",
    };
    
    PipeFactory::create_pipe_from_str(pipe_type, "service_pipe")
}
```

### 3. 动态配置更新

```rust
// 支持运行时配置更新
fn update_pipe_config(new_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 验证新配置
    let _pipe_type = PipeType::from_str(new_type)?;
    
    // 创建新管道
    let new_pipe = PipeFactory::create_pipe_from_str(new_type, "updated_pipe")?;
    
    println!("配置已更新为: {}", new_type);
    println!("新管道容量: {}, 槽大小: {}", new_pipe.capacity(), new_pipe.slot_size());
    
    Ok(())
}
```

## 最佳实践

1. **配置验证**：始终验证从外部源读取的配置
2. **错误处理**：提供清晰的错误信息和回退机制
3. **环境变量**：支持环境变量覆盖配置文件设置
4. **性能考虑**：根据应用需求选择合适的管道类型
5. **文档化**：在配置文件中添加注释说明支持的类型

## 示例代码

完整的示例代码可以在以下文件中找到：

- `examples/string_config_example.rs` - 基本字符串配置使用
- `examples/config_file_example.rs` - 配置文件读取示例
- `examples/pipe_config.toml` - 示例配置文件

## 注意事项

1. 字符串匹配不区分大小写
2. 无效的字符串会返回详细的错误信息
3. 支持的类型列表可通过 `PipeType::supported_types()` 获取
4. 建议在生产环境中使用专门的配置库（如 `serde`, `toml`）进行配置解析