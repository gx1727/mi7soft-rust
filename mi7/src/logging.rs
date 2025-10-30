use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use anyhow::Result;

/// 日志初始化配置
pub struct LogConfig {
    /// 日志目录
    pub log_dir: String,
    /// 日志文件前缀
    pub file_prefix: String,
}

impl LogConfig {
    /// 创建新的日志配置
    pub fn new(file_prefix: impl Into<String>) -> Self {
        Self {
            log_dir: "logs".to_string(),
            file_prefix: file_prefix.into(),
        }
    }

    /// 设置日志目录
    pub fn with_log_dir(mut self, log_dir: impl Into<String>) -> Self {
        self.log_dir = log_dir.into();
        self
    }
}

/// 安全的多进程文件写入器
/// 使用文件锁确保多个进程可以安全地写入同一个日志文件
pub struct SafeFileWriter {
    file: Arc<File>,
    path: PathBuf,
}

impl SafeFileWriter {
    /// 创建新的安全文件写入器
    pub fn new(path: PathBuf) -> io::Result<Self> {
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 以追加模式打开文件
        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        Ok(Self {
            file: Arc::new(file),
            path,
        })
    }
}

impl Write for SafeFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // 获取文件锁
        self.file.lock_exclusive()?;

        // 重新打开文件以确保获取最新的文件指针位置
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        // 写入数据
        let result = file.write(buf);

        // 确保数据写入磁盘
        file.flush()?;

        // 释放文件锁（文件关闭时自动释放）
        drop(file);

        result
    }

    fn flush(&mut self) -> io::Result<()> {
        // 对于追加模式，flush 在 write 中已经处理
        Ok(())
    }
}

impl Clone for SafeFileWriter {
    fn clone(&self) -> Self {
        Self {
            file: Arc::clone(&self.file),
            path: self.path.clone(),
        }
    }
}

impl<'a> MakeWriter<'a> for SafeFileWriter {
    type Writer = SafeFileWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// 初始化日志系统
///
/// 这个函数会：
/// 1. 创建日志目录
/// 2. 设置按日期分割的文件日志
/// 3. 同时输出到控制台和文件
/// 4. 配置日志格式（包含线程ID和线程名）
///
/// # 参数
/// - `config`: 日志配置
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(anyhow::Error)`: 初始化失败
///
/// # 示例
/// ```rust
/// use mi7::logging::{init_logging, LogConfig};
///
/// // 基本用法
/// init_logging(LogConfig::new("my-app"))?;
///
/// // 自定义日志目录
/// init_logging(LogConfig::new("my-app").with_log_dir("custom-logs"))?;
/// ```
pub fn init_logging(config: LogConfig) -> Result<()> {
    // 创建日志目录
    std::fs::create_dir_all(&config.log_dir)?;

    // 创建文件日志 appender（按日期分割）
    let file_appender = rolling::daily(&config.log_dir, &config.file_prefix);
    let (non_blocking, _guard) = non_blocking(file_appender);

    // 初始化 tracing subscriber
    tracing_subscriber::registry()
        .with(
            // 文件日志层
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false) // 文件中不使用颜色
                .with_target(false) // 不显示目标模块
                // .with_thread_ids(true) // 显示线程ID
                .with_thread_names(true), // 显示线程名
        )
        .with(
            // 控制台日志层
            fmt::layer().with_writer(io::stdout).with_ansi(true), // 控制台使用颜色
        )
        .init();

    // 防止 _guard 被提前释放，这里我们故意"泄露"它
    // 这是必要的，因为 _guard 控制着日志写入器的生命周期
    std::mem::forget(_guard);

    Ok(())
}

/// 便捷函数：使用默认配置初始化日志
///
/// 等价于 `init_logging(LogConfig::new(app_name))`
pub fn init_default_logging(app_name: impl Into<String>) -> Result<()> {
    init_logging(LogConfig::new(app_name))
}

/// 初始化安全的多进程日志系统
///
/// 这个函数专门用于多个进程需要写入同一个日志文件的场景。
/// 使用文件锁确保写入安全，避免日志行交错或截断。
///
/// # 参数
/// - `config`: 日志配置
///
/// # 返回
/// - `Ok(())`: 初始化成功
/// - `Err(anyhow::Error)`: 初始化失败
///
/// # 示例
/// ```rust
/// use mi7::logging::{init_safe_multiprocess_logging, LogConfig};
///
/// // 多个 worker 进程使用相同的日志文件
/// init_safe_multiprocess_logging(LogConfig::new("workers"))?;
/// ```
pub fn init_safe_multiprocess_logging(config: LogConfig) -> Result<()> {
    // 创建日志目录
    std::fs::create_dir_all(&config.log_dir)?;

    // 构建日志文件路径
    let now = chrono::Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let log_file_path =
        PathBuf::from(&config.log_dir).join(format!("{}.{}.log", config.file_prefix, date_str));

    // 创建安全的文件写入器
    let safe_writer = SafeFileWriter::new(log_file_path)?;

    // 初始化 tracing subscriber
    tracing_subscriber::registry()
        .with(
            // 文件日志层 - 使用安全的多进程写入器
            fmt::layer()
                .with_writer(safe_writer)
                .with_ansi(false) // 文件中不使用颜色
                .with_target(false) // 不显示目标模块
                .with_thread_ids(true) // 显示线程ID
                .with_thread_names(true), // 显示线程名
        )
        .with(
            // 控制台日志层
            fmt::layer().with_writer(io::stdout).with_ansi(true), // 控制台使用颜色
        )
        .init();

    Ok(())
}

/// 便捷函数：使用默认配置初始化安全的多进程日志
///
/// 等价于 `init_safe_multiprocess_logging(LogConfig::new(app_name))`
pub fn init_safe_multiprocess_default_logging(
    app_name: impl Into<String>,
) -> Result<()> {
    init_safe_multiprocess_logging(LogConfig::new(app_name))
}
