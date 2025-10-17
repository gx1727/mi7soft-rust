//! 日志基础设施

use tracing::Level;
use tracing_subscriber;

pub struct Logger;

impl Logger {
    pub fn init(level: Level) {
        tracing_subscriber::fmt().with_max_level(level).init();
    }
}
