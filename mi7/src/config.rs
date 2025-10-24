use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 全局配置实例
static CONFIG: OnceLock<Config> = OnceLock::new();

/// MI7 系统配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 共享内存配置
    pub shared_memory: SharedMemoryConfig,
    /// 日志配置
    pub logging: LoggingConfig,
    /// HTTP 服务配置
    pub http: HttpConfig,
    /// 队列配置
    pub queue: QueueConfig,
}

/// 共享内存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMemoryConfig {
    /// 共享内存名称
    pub name: String,
    /// 槽位大小（字节）
    pub slot_size: usize,
    /// 槽位数量
    pub slot_count: usize,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志文件路径
    pub log_path: PathBuf,
    /// 日志文件名前缀
    pub file_prefix: String,
    /// 是否启用控制台输出
    pub console_output: bool,
    /// 日志级别 (trace, debug, info, warn, error)
    pub level: String,
}

/// HTTP 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// HTTP 服务端口
    pub port: u16,
    /// 绑定地址
    pub bind_address: String,
    /// 请求超时时间（秒）
    pub timeout_seconds: u64,
    /// 最大并发连接数
    pub max_connections: usize,
}

/// 队列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// 队列容量
    pub capacity: usize,
    /// 队列名称
    pub name: String,
    /// 是否启用持久化
    pub persistent: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shared_memory: SharedMemoryConfig::default(),
            logging: LoggingConfig::default(),
            http: HttpConfig::default(),
            queue: QueueConfig::default(),
        }
    }
}

impl Default for SharedMemoryConfig {
    fn default() -> Self {
        Self {
            name: "mi7_shared_memory".to_string(),
            slot_size: 1024,
            slot_count: 100,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_path: PathBuf::from("./logs"),
            file_prefix: "workers".to_string(),
            console_output: true,
            level: "info".to_string(),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            bind_address: "0.0.0.0".to_string(),
            timeout_seconds: 30,
            max_connections: 1000,
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            name: "task_queue".to_string(),
            persistent: false,
        }
    }
}

impl Config {
    /// 从配置文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::FileRead(e.to_string()))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;
        
        // 确保目录存在
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::FileWrite(e.to_string()))?;
        }
        
        fs::write(path.as_ref(), content)
            .map_err(|e| ConfigError::FileWrite(e.to_string()))?;
        
        Ok(())
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证共享内存配置
        if self.shared_memory.name.is_empty() {
            return Err(ConfigError::Validation("共享内存名称不能为空".to_string()));
        }
        if self.shared_memory.slot_size == 0 {
            return Err(ConfigError::Validation("槽位大小必须大于0".to_string()));
        }
        if self.shared_memory.slot_count == 0 {
            return Err(ConfigError::Validation("槽位数量必须大于0".to_string()));
        }

        // 验证HTTP配置
        if self.http.port == 0 {
            return Err(ConfigError::Validation("HTTP端口必须大于0".to_string()));
        }
        if self.http.bind_address.is_empty() {
            return Err(ConfigError::Validation("绑定地址不能为空".to_string()));
        }

        // 验证队列配置
        if self.queue.capacity == 0 {
            return Err(ConfigError::Validation("队列容量必须大于0".to_string()));
        }
        if self.queue.name.is_empty() {
            return Err(ConfigError::Validation("队列名称不能为空".to_string()));
        }

        // 验证日志级别
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(ConfigError::Validation(
                format!("无效的日志级别: {}，有效值: {:?}", self.logging.level, valid_levels)
            ));
        }

        Ok(())
    }
}

/// 配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("文件读取错误: {0}")]
    FileRead(String),
    #[error("文件写入错误: {0}")]
    FileWrite(String),
    #[error("配置解析错误: {0}")]
    Parse(String),
    #[error("配置序列化错误: {0}")]
    Serialize(String),
    #[error("配置验证错误: {0}")]
    Validation(String),
}

/// 初始化全局配置
pub fn init_config() -> Result<(), ConfigError> {
    let config = load_config()?;
    config.validate()?;
    
    CONFIG.set(config).map_err(|_| {
        ConfigError::Validation("配置已经初始化".to_string())
    })?;
    
    Ok(())
}

/// 从文件或默认值加载配置
pub fn load_config() -> Result<Config, ConfigError> {
    let config_paths = [
        "config.toml",
        "mi7.toml",
        "./config/config.toml",
        "./config/mi7.toml",
    ];

    // 尝试从配置文件加载
    for path in &config_paths {
        if Path::new(path).exists() {
            println!("从配置文件加载: {}", path);
            return Config::load_from_file(path);
        }
    }

    // 如果没有找到配置文件，使用默认配置
    println!("未找到配置文件，使用默认配置");
    Ok(Config::default())
}

/// 获取全局配置实例
pub fn get_config() -> &'static Config {
    CONFIG.get().expect("配置未初始化，请先调用 init_config()")
}

/// 获取共享内存配置
pub fn get_shared_memory_config() -> &'static SharedMemoryConfig {
    &get_config().shared_memory
}

/// 获取日志配置
pub fn get_logging_config() -> &'static LoggingConfig {
    &get_config().logging
}

/// 获取HTTP配置
pub fn get_http_config() -> &'static HttpConfig {
    &get_config().http
}

/// 获取队列配置
pub fn get_queue_config() -> &'static QueueConfig {
    &get_config().queue
}

/// 便捷方法：获取共享内存名称
pub fn get_shared_memory_name() -> &'static str {
    &get_shared_memory_config().name
}

/// 便捷方法：获取槽位大小
pub fn get_slot_size() -> usize {
    get_shared_memory_config().slot_size
}

/// 便捷方法：获取槽位数量
pub fn get_slot_count() -> usize {
    get_shared_memory_config().slot_count
}

/// 便捷方法：获取日志路径
pub fn get_log_path() -> &'static PathBuf {
    &get_logging_config().log_path
}

/// 便捷方法：获取HTTP端口
pub fn get_http_port() -> u16 {
    get_http_config().port
}

/// 便捷方法：获取HTTP绑定地址
pub fn get_http_bind_address() -> &'static str {
    &get_http_config().bind_address
}

/// 便捷方法：获取队列容量
pub fn get_queue_capacity() -> usize {
    get_queue_config().capacity
}

/// 便捷方法：获取队列名称
pub fn get_queue_name() -> &'static str {
    &get_queue_config().name
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.shared_memory.name, "mi7_shared_memory");
        assert_eq!(config.shared_memory.slot_size, 1024);
        assert_eq!(config.http.port, 8080);
        assert_eq!(config.queue.capacity, 100);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        // 测试无效配置
        config.shared_memory.name = "".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_save_load() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.toml");
        
        let config = Config::default();
        config.save_to_file(&config_path).unwrap();
        
        let loaded_config = Config::load_from_file(&config_path).unwrap();
        assert_eq!(config.shared_memory.name, loaded_config.shared_memory.name);
        assert_eq!(config.http.port, loaded_config.http.port);
    }
}