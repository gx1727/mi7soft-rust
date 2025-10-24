# MI7 配置管理指南

## 概述

MI7 配置模块提供了一个统一的配置管理系统，支持从配置文件加载设置，并提供便捷的访问方法。

## 配置文件格式

配置文件使用 TOML 格式，支持以下配置项：

### 共享内存配置 (shared_memory)
- `name`: 共享内存名称
- `slot_size`: 槽位大小（字节）
- `slot_count`: 槽位数量

### 日志配置 (logging)
- `log_path`: 日志文件路径
- `file_prefix`: 日志文件名前缀
- `console_output`: 是否输出到控制台
- `level`: 日志级别 (trace, debug, info, warn, error)

### HTTP 配置 (http)
- `port`: HTTP 服务端口
- `bind_address`: 绑定地址
- `timeout_seconds`: 请求超时时间
- `max_connections`: 最大并发连接数

### 队列配置 (queue)
- `capacity`: 队列容量
- `name`: 队列名称
- `persistent`: 是否启用持久化

## 使用方法

### 1. 初始化配置

```rust
use mi7::config::{init_config, ConfigError};

fn main() -> Result<(), ConfigError> {
    // 初始化配置（会自动查找配置文件或使用默认值）
    init_config()?;
    
    // 现在可以使用配置了
    Ok(())
}
```

### 2. 获取配置值

```rust
use mi7::config::{
    get_config,
    get_shared_memory_name,
    get_slot_size,
    get_http_port,
    get_log_path,
};

// 获取完整配置
let config = get_config();
println!("配置: {:?}", config);

// 获取特定配置值
let memory_name = get_shared_memory_name();
let slot_size = get_slot_size();
let http_port = get_http_port();
let log_path = get_log_path();

println!("共享内存名称: {}", memory_name);
println!("槽位大小: {}", slot_size);
println!("HTTP端口: {}", http_port);
println!("日志路径: {:?}", log_path);
```

### 3. 获取分类配置

```rust
use mi7::config::{
    get_shared_memory_config,
    get_logging_config,
    get_http_config,
    get_queue_config,
};

// 获取共享内存配置
let shm_config = get_shared_memory_config();
println!("共享内存配置: {:?}", shm_config);

// 获取日志配置
let log_config = get_logging_config();
println!("日志配置: {:?}", log_config);

// 获取HTTP配置
let http_config = get_http_config();
println!("HTTP配置: {:?}", http_config);

// 获取队列配置
let queue_config = get_queue_config();
println!("队列配置: {:?}", queue_config);
```

### 4. 创建和保存配置

```rust
use mi7::config::{Config, ConfigError};

fn create_custom_config() -> Result<(), ConfigError> {
    let mut config = Config::default();
    
    // 修改配置
    config.shared_memory.name = "my_custom_memory".to_string();
    config.shared_memory.slot_size = 2048;
    config.http.port = 9090;
    
    // 验证配置
    config.validate()?;
    
    // 保存到文件
    config.save_to_file("my_config.toml")?;
    
    Ok(())
}
```

### 5. 从特定文件加载配置

```rust
use mi7::config::{Config, ConfigError};

fn load_custom_config() -> Result<(), ConfigError> {
    let config = Config::load_from_file("my_config.toml")?;
    config.validate()?;
    
    println!("加载的配置: {:?}", config);
    Ok(())
}
```

## 配置文件查找顺序

配置模块会按以下顺序查找配置文件：

1. `config.toml`
2. `mi7.toml`
3. `./config/config.toml`
4. `./config/mi7.toml`

如果找不到任何配置文件，将使用默认配置。

## 添加新的配置项

要添加新的配置项，请按以下步骤操作：

### 1. 修改配置结构体

在 `mi7/src/config.rs` 中添加新的配置结构体或字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyNewConfig {
    pub my_setting: String,
    pub my_number: u32,
}

// 在 Config 结构体中添加
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // ... 现有字段
    pub my_new_config: MyNewConfig,
}
```

### 2. 实现默认值

```rust
impl Default for MyNewConfig {
    fn default() -> Self {
        Self {
            my_setting: "default_value".to_string(),
            my_number: 42,
        }
    }
}

// 在 Config::default() 中添加
impl Default for Config {
    fn default() -> Self {
        Self {
            // ... 现有字段
            my_new_config: MyNewConfig::default(),
        }
    }
}
```

### 3. 添加访问方法

```rust
/// 获取新配置
pub fn get_my_new_config() -> &'static MyNewConfig {
    &get_config().my_new_config
}

/// 便捷方法：获取特定设置
pub fn get_my_setting() -> &'static str {
    &get_my_new_config().my_setting
}

pub fn get_my_number() -> u32 {
    get_my_new_config().my_number
}
```

### 4. 在 lib.rs 中导出

```rust
pub use config::{
    // ... 现有导出
    MyNewConfig,
    get_my_new_config,
    get_my_setting,
    get_my_number,
};
```

### 5. 添加验证（可选）

在 `Config::validate()` 方法中添加新配置的验证逻辑：

```rust
impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // ... 现有验证
        
        // 验证新配置
        if self.my_new_config.my_setting.is_empty() {
            return Err(ConfigError::Validation("my_setting 不能为空".to_string()));
        }
        
        Ok(())
    }
}
```

## 错误处理

配置模块定义了 `ConfigError` 枚举来处理各种错误情况：

- `FileRead`: 文件读取错误
- `FileWrite`: 文件写入错误
- `Parse`: 配置解析错误
- `Serialize`: 配置序列化错误
- `Validation`: 配置验证错误

## 测试

配置模块包含了完整的单元测试，可以运行：

```bash
cargo test config
```

## 示例项目集成

在你的项目中使用配置模块：

```rust
// main.rs
use mi7::config::{init_config, get_http_port, get_shared_memory_name};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置
    init_config()?;
    
    // 使用配置
    let port = get_http_port();
    let memory_name = get_shared_memory_name();
    
    println!("启动HTTP服务器，端口: {}", port);
    println!("使用共享内存: {}", memory_name);
    
    Ok(())
}
```

这样，你就可以轻松地管理和扩展 MI7 系统的配置了！