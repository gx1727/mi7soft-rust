use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// 全局配置实例
static CONFIG: OnceLock<Config> = OnceLock::new();

/// MI7 系统配置结构 - 基于 HashMap 的灵活配置系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 所有配置段的映射，每个段包含键值对
    #[serde(flatten)]
    pub sections: HashMap<String, HashMap<String, ConfigValue>>,
}

/// 配置值类型，支持多种数据类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl ConfigValue {
    /// 获取字符串值
    pub fn as_string(&self) -> Option<String> {
        match self {
            ConfigValue::String(s) => Some(s.clone()),
            ConfigValue::Integer(i) => Some(i.to_string()),
            ConfigValue::Float(f) => Some(f.to_string()),
            ConfigValue::Boolean(b) => Some(b.to_string()),
        }
    }

    /// 获取整数值
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(i) => Some(*i),
            ConfigValue::String(s) => s.parse().ok(),
            ConfigValue::Float(f) => Some(*f as i64),
            ConfigValue::Boolean(b) => Some(if *b { 1 } else { 0 }),
        }
    }

    /// 获取浮点数值
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Integer(i) => Some(*i as f64),
            ConfigValue::String(s) => s.parse().ok(),
            ConfigValue::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        }
    }

    /// 获取布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            ConfigValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" | "on" => Some(true),
                "false" | "no" | "0" | "off" => Some(false),
                _ => None,
            },
            ConfigValue::Integer(i) => Some(*i != 0),
            ConfigValue::Float(f) => Some(*f != 0.0),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut sections = HashMap::new();

        // 共享内存配置
        let mut shared_memory = HashMap::new();
        shared_memory.insert("name".to_string(), ConfigValue::String("mi7_daemon_queue".to_string()));
        sections.insert("shared_memory".to_string(), shared_memory);

        // 队列配置
        let mut queue = HashMap::new();
        queue.insert("capacity".to_string(), ConfigValue::Integer(200));
        queue.insert("name".to_string(), ConfigValue::String("pipe_status_test".to_string()));
        queue.insert("persistent".to_string(), ConfigValue::Boolean(false));
        sections.insert("queue".to_string(), queue);

        // 入口配置
        let mut entry = HashMap::new();
        entry.insert("interface_name".to_string(), ConfigValue::String("entry_resp_pipe".to_string()));
        entry.insert("interface_type".to_string(), ConfigValue::String("default".to_string()));
        entry.insert("log_level".to_string(), ConfigValue::String("info".to_string()));
        sections.insert("entry".to_string(), entry);

        // 工作者配置
        let mut worker = HashMap::new();
        worker.insert("interface_name".to_string(), ConfigValue::String("work_req_pipe".to_string()));
        worker.insert("interface_type".to_string(), ConfigValue::String("large".to_string()));
        worker.insert("log_prefix".to_string(), ConfigValue::String("workers".to_string()));
        worker.insert("log_level".to_string(), ConfigValue::String("info".to_string()));
        sections.insert("worker".to_string(), worker);

        // 日志配置
        let mut logging = HashMap::new();
        logging.insert("log_path".to_string(), ConfigValue::String("./logs".to_string()));
        logging.insert("log_prefix".to_string(), ConfigValue::String("mi7".to_string()));
        logging.insert("console_output".to_string(), ConfigValue::Boolean(true));
        logging.insert("level".to_string(), ConfigValue::String("info".to_string()));
        sections.insert("logging".to_string(), logging);

        // HTTP 配置
        let mut http = HashMap::new();
        http.insert("port".to_string(), ConfigValue::Integer(8888));
        http.insert("bind_address".to_string(), ConfigValue::String("0.0.0.0".to_string()));
        http.insert("timeout_seconds".to_string(), ConfigValue::Integer(30));
        http.insert("max_connections".to_string(), ConfigValue::Integer(1000));
        sections.insert("http".to_string(), http);

        Self { sections }
    }
}

impl Config {
    /// 从配置文件加载配置
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content =
            fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::FileRead(e.to_string()))?;

        // 解析 TOML 为通用的 Value 类型
        let toml_value: toml::Value = toml::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        
        let mut config = Config::default();
        
        // 将 TOML 数据转换为 HashMap 结构
        if let toml::Value::Table(table) = toml_value {
            for (section_name, section_value) in table {
                if let toml::Value::Table(section_table) = section_value {
                    let mut section_map = HashMap::new();
                    
                    for (key, value) in section_table {
                        let config_value = match value {
                            toml::Value::String(s) => ConfigValue::String(s),
                            toml::Value::Integer(i) => ConfigValue::Integer(i),
                            toml::Value::Float(f) => ConfigValue::Float(f),
                            toml::Value::Boolean(b) => ConfigValue::Boolean(b),
                            _ => {
                                eprintln!("警告: 不支持的配置值类型，跳过 {}.{}", section_name, key);
                                continue;
                            }
                        };
                        section_map.insert(key, config_value);
                    }
                    
                    config.sections.insert(section_name, section_map);
                }
            }
        }
        
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        // 将 HashMap 结构转换为 TOML Value
        let mut toml_table = toml::value::Table::new();
        
        for (section_name, section_map) in &self.sections {
            let mut section_table = toml::value::Table::new();
            
            for (key, value) in section_map {
                let toml_value = match value {
                    ConfigValue::String(s) => toml::Value::String(s.clone()),
                    ConfigValue::Integer(i) => toml::Value::Integer(*i),
                    ConfigValue::Float(f) => toml::Value::Float(*f),
                    ConfigValue::Boolean(b) => toml::Value::Boolean(*b),
                };
                section_table.insert(key.clone(), toml_value);
            }
            
            toml_table.insert(section_name.clone(), toml::Value::Table(section_table));
        }
        
        let content = toml::to_string_pretty(&toml::Value::Table(toml_table))
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;

        // 确保目录存在
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::FileWrite(e.to_string()))?;
        }

        fs::write(path.as_ref(), content).map_err(|e| ConfigError::FileWrite(e.to_string()))?;

        Ok(())
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), ConfigError> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];

        // 验证entry配置
        if let Some(entry_section) = self.sections.get("entry") {
            if let Some(interface_name) = entry_section.get("interface_name") {
                if let Some(name) = interface_name.as_string() {
                    if name.is_empty() {
                        return Err(ConfigError::Validation("entry.interface_name 不能为空".to_string()));
                    }
                }
            }

            if let Some(log_level) = entry_section.get("log_level") {
                if let Some(level) = log_level.as_string() {
                    if !valid_levels.contains(&level.as_str()) {
                        return Err(ConfigError::Validation(format!(
                            "无效的 entry.log_level: {}，有效值: {:?}",
                            level, valid_levels
                        )));
                    }
                }
            }
        }

        // 验证worker配置
        if let Some(worker_section) = self.sections.get("worker") {
            if let Some(interface_name) = worker_section.get("interface_name") {
                if let Some(name) = interface_name.as_string() {
                    if name.is_empty() {
                        return Err(ConfigError::Validation("worker.interface_name 不能为空".to_string()));
                    }
                }
            }

            if let Some(log_level) = worker_section.get("log_level") {
                if let Some(level) = log_level.as_string() {
                    if !valid_levels.contains(&level.as_str()) {
                        return Err(ConfigError::Validation(format!(
                            "无效的 worker.log_level: {}，有效值: {:?}",
                            level, valid_levels
                        )));
                    }
                }
            }
        }

        // 验证logging配置
        if let Some(logging_section) = self.sections.get("logging") {
            if let Some(log_level) = logging_section.get("level") {
                if let Some(level) = log_level.as_string() {
                    if !valid_levels.contains(&level.as_str()) {
                        return Err(ConfigError::Validation(format!(
                            "无效的 logging.level: {}，有效值: {:?}",
                            level, valid_levels
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取配置值
    pub fn get(&self, section: &str, key: &str) -> Option<&ConfigValue> {
        self.sections.get(section)?.get(key)
    }

    /// 设置配置值
    pub fn set(&mut self, section: &str, key: &str, value: ConfigValue) {
        self.sections
            .entry(section.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
    }

    /// 获取所有段名
    pub fn get_sections(&self) -> Vec<&String> {
        self.sections.keys().collect()
    }

    /// 获取指定段的所有键
    pub fn get_keys(&self, section: &str) -> Option<Vec<&String>> {
        self.sections.get(section).map(|s| s.keys().collect())
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
/// * `section` - 配置段名称 (如 "shared_memory", "logging", "http", "queue", "worker", "entry")
/// * `key` - 配置键名称 (如 "name", "level", "port", "interface_name")
///
/// # 示例
/// ```rust
/// let queue_name = config::string("shared_memory", "name");
/// let log_level = config::string("logging", "level");
/// let hello_value = config::string("worker", "hello"); // 动态配置项
/// ```
pub fn string(section: &str, key: &str) -> String {
    let config = get_config();
    
    config
        .get(section, key)
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| {
            // 如果找不到配置项，返回空字符串而不是 panic
            eprintln!("警告: 未找到配置项 {}.{}", section, key);
            String::new()
        })
}

/// 通用配置读取函数：获取字符串值，带默认值
///
/// # 参数
/// * `section` - 配置段名称
/// * `key` - 配置键名称
/// * `default` - 默认值
///
/// # 示例
/// ```rust
/// let hello_value = config::string_or("worker", "hello", "world");
/// ```
pub fn string_or(section: &str, key: &str, default: &str) -> String {
    let config = get_config();
    
    config
        .get(section, key)
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| default.to_string())
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
    
    config
        .get(section, key)
        .and_then(|v| v.as_int())
        .unwrap_or_else(|| {
            eprintln!("警告: 未找到配置项 {}.{} 或无法转换为整数", section, key);
            0
        })
}

/// 通用配置读取函数：获取整数值，带默认值
///
/// # 参数
/// * `section` - 配置段名称
/// * `key` - 配置键名称
/// * `default` - 默认值
///
/// # 示例
/// ```rust
/// let port = config::int_or("http", "port", 8080);
/// ```
pub fn int_or(section: &str, key: &str, default: i64) -> i64 {
    let config = get_config();
    
    config
        .get(section, key)
        .and_then(|v| v.as_int())
        .unwrap_or(default)
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
    
    config
        .get(section, key)
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| {
            eprintln!("警告: 未找到配置项 {}.{} 或无法转换为布尔值", section, key);
            false
        })
}

/// 通用配置读取函数：获取布尔值，带默认值
///
/// # 参数
/// * `section` - 配置段名称
/// * `key` - 配置键名称
/// * `default` - 默认值
///
/// # 示例
/// ```rust
/// let debug_enabled = config::bool_or("logging", "debug", false);
/// ```
pub fn bool_or(section: &str, key: &str, default: bool) -> bool {
    let config = get_config();
    
    config
        .get(section, key)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// 便捷的配置访问结构体，提供简化的配置访问方法
pub struct ConfigAccessor;

impl ConfigAccessor {
    /// 获取字符串配置值
    /// 
    /// # 参数
    /// * `key` - 配置键，格式为 "section.key" 或直接为 "key"（默认在 worker 段中查找）
    /// 
    /// # 示例
    /// ```rust
    /// let config = config::ConfigAccessor;
    /// let hello = config.string("hello");  // 在 worker 段中查找
    /// let name = config.string("shared_memory.name");  // 在 shared_memory 段中查找
    /// ```
    pub fn string(&self, key: &str) -> String {
        if let Some((section, key)) = key.split_once('.') {
            string(section, key)
        } else {
            // 默认在 worker 段中查找
            string("worker", key)
        }
    }
    
    /// 获取整数配置值
    pub fn int(&self, key: &str) -> i64 {
        if let Some((section, key)) = key.split_once('.') {
            int(section, key)
        } else {
            int("worker", key)
        }
    }
    
    /// 获取布尔配置值
    pub fn bool(&self, key: &str) -> bool {
        if let Some((section, key)) = key.split_once('.') {
            bool(section, key)
        } else {
            bool("worker", key)
        }
    }
    
    /// 获取字符串配置值，带默认值
    pub fn string_or(&self, key: &str, default: &str) -> String {
        if let Some((section, key)) = key.split_once('.') {
            string_or(section, key, default)
        } else {
            string_or("worker", key, default)
        }
    }
    
    /// 获取整数配置值，带默认值
    pub fn int_or(&self, key: &str, default: i64) -> i64 {
        if let Some((section, key)) = key.split_once('.') {
            int_or(section, key, default)
        } else {
            int_or("worker", key, default)
        }
    }
    
    /// 获取布尔配置值，带默认值
    pub fn bool_or(&self, key: &str, default: bool) -> bool {
        if let Some((section, key)) = key.split_once('.') {
            bool_or(section, key, default)
        } else {
            bool_or("worker", key, default)
        }
    }
}

/// 全局配置访问器实例
pub static CONFIG_ACCESSOR: ConfigAccessor = ConfigAccessor;
