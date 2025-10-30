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
    /// 队列配置
    pub queue: QueueConfig,
    /// 入口配置
    pub entry: EntryConfig,
    /// 工作者配置
    pub worker: WorkerConfig,
}

/// 共享内存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMemoryConfig {
    /// 共享内存队列名称
    pub name: String,
}

/// 队列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// 队列容量
    pub capacity: i64,
    /// 队列名称
    pub name: String,
    /// 是否持久化
    pub persistent: bool,
}

/// 入口配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryConfig {
    /// 接口队列名称
    pub interface_name: String,
    /// 接口队列类型
    pub interface_type: String,
    /// 日志等级
    pub log_level: String,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// 接口队列名称
    pub interface_name: String,
    /// 接口队列类型
    pub interface_type: String,
    /// 日志文件名前缀
    pub log_prefix: String,
    /// 日志等级
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shared_memory: SharedMemoryConfig::default(),
            queue: QueueConfig::default(),
            entry: EntryConfig::default(),
            worker: WorkerConfig::default(),
        }
    }
}

impl Default for SharedMemoryConfig {
    fn default() -> Self {
        Self {
            name: "mi7_daemon_queue".to_string(),
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            capacity: 200,
            name: "pipe_status_test".to_string(),
            persistent: false,
        }
    }
}

impl Default for EntryConfig {
    fn default() -> Self {
        Self {
            interface_name: "entry_resp_pipe".to_string(),
            interface_type: "default".to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            interface_name: "entry_resp_pipe".to_string(),
            interface_type: "default".to_string(),
            log_prefix: "workers".to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl Config {
    /// 从配置文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content =
            fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::FileRead(e.to_string()))?;

        let config: Config =
            toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;

        // 确保目录存在
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::FileWrite(e.to_string()))?;
        }

        fs::write(path.as_ref(), content).map_err(|e| ConfigError::FileWrite(e.to_string()))?;

        Ok(())
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证entry配置
        if self.entry.interface_name.is_empty() {
            return Err(ConfigError::Validation("接口队列名称不能为空".to_string()));
        }
        if self.entry.interface_type.is_empty() {
            return Err(ConfigError::Validation("接口队列类型不能为空".to_string()));
        }

        // 验证worker配置
        if self.worker.interface_name.is_empty() {
            return Err(ConfigError::Validation("接口队列名称不能为空".to_string()));
        }
        if self.worker.interface_type.is_empty() {
            return Err(ConfigError::Validation("接口队列类型不能为空".to_string()));
        }
        if self.worker.log_prefix.is_empty() {
            return Err(ConfigError::Validation(
                "日志文件名前缀不能为空".to_string(),
            ));
        }

        // 验证日志级别
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.entry.log_level.as_str()) {
            return Err(ConfigError::Validation(format!(
                "无效的日志级别: {}，有效值: {:?}",
                self.entry.log_level, valid_levels
            )));
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

    CONFIG
        .set(config)
        .map_err(|_| ConfigError::Validation("配置已经初始化".to_string()))?;

    Ok(())
}

/// 从文件或默认值加载配置
pub fn load_config() -> Result<Config, ConfigError> {
    let config_paths = ["config.toml", "./config/config.toml"];

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

/// 通用配置读取函数：获取字符串值
///
/// # 参数
/// * `section` - 配置段名称 (如 "shared_memory", "logging", "http", "queue")
/// * `key` - 配置键名称 (如 "name", "level", "port")
///
/// # 示例
/// ```rust
/// let queue_name = config::string("shared_memory", "name");
/// let log_level = config::string("logging", "level");
/// ```
pub fn string(section: &str, key: &str) -> String {
    let config = get_config();

    match section {
        "shared_memory" => match key {
            "name" => config.shared_memory.name.clone(),
            _ => panic!("未知的 shared_memory 配置键: {}", key),
        },
        "queue" => match key {
            "name" => config.queue.name.clone(),
            _ => panic!("未知的 queue 配置键: {}", key),
        },
        "entry" => match key {
            "interface_name" => config.entry.interface_name.clone(),
            "interface_type" => config.entry.interface_type.clone(),
            "log_level" => config.entry.log_level.clone(),
            _ => panic!("未知的 entry 配置键: {}", key),
        },
        "worker" => match key {
            "interface_name" => config.worker.interface_name.clone(),
            "interface_type" => config.worker.interface_type.clone(),
            "log_prefix" => config.worker.log_prefix.clone(),
            "log_level" => config.worker.log_level.clone(),
            _ => panic!("未知的 entry 配置键: {}", key),
        },
        _ => panic!("未知的配置段: {}", section),
    }
}

/// 通用配置读取函数：获取整数值
///
/// # 参数
/// * `section` - 配置段名称 (如 "shared_memory", "http", "queue")
/// * `key` - 配置键名称 (如 "slot_size", "port", "capacity")
///
/// # 示例
/// ```rust
/// let slot_size = config::int("shared_memory", "slot_size");
/// let http_port = config::int("http", "port");
/// ```
pub fn int(section: &str, key: &str) -> i64 {
    let config = get_config();

    match section {
        "queue" => match key {
            "capacity" => config.queue.capacity,
            _ => panic!("未知的 queue 配置键: {}", key),
        },
        "entry" => match key {
            _ => panic!("未知的 entry 配置键: {}", key),
        },
        "worker" => match key {
            _ => panic!("未知的 worker 配置键: {}", key),
        },
        _ => panic!("未知的配置段: {}", section),
    }
}

/// 通用配置读取函数：获取布尔值
///
/// # 参数
/// * `section` - 配置段名称 (如 "logging", "queue")
/// * `key` - 配置键名称 (如 "console_output", "persistent")
///
/// # 示例
/// ```rust
/// let console_output = config::bool("logging", "console_output");
/// let persistent = config::bool("queue", "persistent");
/// ```
pub fn bool(section: &str, key: &str) -> bool {
    let config = get_config();

    match section {
        "entry" => match key {
            _ => panic!("未知的 entry 配置键: {}", key),
        },
        "worker" => match key {
            _ => panic!("未知的 worker 配置键: {}", key),
        },
        _ => panic!("未知的配置段: {}", section),
    }
}
