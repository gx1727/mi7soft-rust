use std::time::{Duration, Instant};
use sysinfo::System;

/// 系统信息工具
pub struct SystemInfo {
    system: System,
}

impl SystemInfo {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }
    
    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }
    
    pub fn get_memory_usage(&self) -> (u64, u64) {
        (self.system.used_memory(), self.system.total_memory())
    }
    
    pub fn get_cpu_usage(&self) -> f32 {
        // 获取全局CPU使用率
        self.system.global_cpu_info().cpu_usage()
    }
    
    pub fn get_process_count(&self) -> usize {
        self.system.processes().len()
    }
    
    pub fn print_system_info(&self) {
        let (used_mem, total_mem) = self.get_memory_usage();
        println!("系统信息:");
        println!("  内存使用: {} MB / {} MB ({:.1}%)", 
                 used_mem / 1024 / 1024, 
                 total_mem / 1024 / 1024,
                 (used_mem as f64 / total_mem as f64) * 100.0);
        println!("  CPU使用率: {:.1}%", self.get_cpu_usage());
        println!("  进程数量: {}", self.get_process_count());
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// 性能测试工具
pub struct PerformanceTester {
    start_time: Option<Instant>,
    measurements: Vec<Duration>,
}

impl PerformanceTester {
    pub fn new() -> Self {
        Self {
            start_time: None,
            measurements: Vec::new(),
        }
    }
    
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }
    
    pub fn stop(&mut self) -> Option<Duration> {
        if let Some(start) = self.start_time.take() {
            let duration = start.elapsed();
            self.measurements.push(duration);
            Some(duration)
        } else {
            None
        }
    }
    
    pub fn get_average(&self) -> Option<Duration> {
        if self.measurements.is_empty() {
            None
        } else {
            let total: Duration = self.measurements.iter().sum();
            Some(total / self.measurements.len() as u32)
        }
    }
    
    pub fn get_min(&self) -> Option<Duration> {
        self.measurements.iter().min().copied()
    }
    
    pub fn get_max(&self) -> Option<Duration> {
        self.measurements.iter().max().copied()
    }
    
    pub fn get_measurements(&self) -> &[Duration] {
        &self.measurements
    }
    
    pub fn clear(&mut self) {
        self.measurements.clear();
        self.start_time = None;
    }
    
    pub fn print_statistics(&self) {
        if self.measurements.is_empty() {
            println!("没有性能测量数据");
            return;
        }
        
        println!("性能统计:");
        println!("  测量次数: {}", self.measurements.len());
        if let Some(avg) = self.get_average() {
            println!("  平均时间: {:?}", avg);
        }
        if let Some(min) = self.get_min() {
            println!("  最短时间: {:?}", min);
        }
        if let Some(max) = self.get_max() {
            println!("  最长时间: {:?}", max);
        }
        
        // 计算标准差
        if let Some(avg) = self.get_average() {
            let variance: f64 = self.measurements.iter()
                .map(|&d| {
                    let diff = d.as_nanos() as f64 - avg.as_nanos() as f64;
                    diff * diff
                })
                .sum::<f64>() / self.measurements.len() as f64;
            let std_dev = variance.sqrt();
            println!("  标准差: {:.2} ns", std_dev);
        }
    }
}

impl Default for PerformanceTester {
    fn default() -> Self {
        Self::new()
    }
}

/// 内存使用情况监控
pub struct MemoryMonitor {
    initial_usage: u64,
    system_info: SystemInfo,
}

impl MemoryMonitor {
    pub fn new() -> Self {
        let system_info = SystemInfo::new();
        let (initial_usage, _) = system_info.get_memory_usage();
        
        Self {
            initial_usage,
            system_info,
        }
    }
    
    pub fn get_memory_delta(&mut self) -> i64 {
        self.system_info.refresh();
        let (current_usage, _) = self.system_info.get_memory_usage();
        current_usage as i64 - self.initial_usage as i64
    }
    
    pub fn print_memory_delta(&mut self) {
        let delta = self.get_memory_delta();
        if delta > 0 {
            println!("内存增加: {} MB", delta / 1024 / 1024);
        } else if delta < 0 {
            println!("内存减少: {} MB", (-delta) / 1024 / 1024);
        } else {
            println!("内存使用无变化");
        }
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// 数据生成工具
pub struct DataGenerator;

impl DataGenerator {
    /// 生成随机字节数据
    pub fn random_bytes(size: usize) -> Vec<u8> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut data = Vec::with_capacity(size);
        let mut hasher = DefaultHasher::new();
        
        for i in 0..size {
            i.hash(&mut hasher);
            data.push((hasher.finish() & 0xFF) as u8);
        }
        
        data
    }
    
    /// 生成测试字符串
    pub fn test_string(length: usize) -> String {
        let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut result = String::with_capacity(length);
        
        for i in 0..length {
            let char_index = i % chars.len();
            result.push(chars.chars().nth(char_index).unwrap());
        }
        
        result
    }
    
    /// 生成数字序列
    pub fn number_sequence(count: usize) -> Vec<u64> {
        (0..count as u64).collect()
    }
    
    /// 生成重复模式数据
    pub fn pattern_data(pattern: &[u8], total_size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(total_size);
        let mut pattern_index = 0;
        
        for _ in 0..total_size {
            data.push(pattern[pattern_index]);
            pattern_index = (pattern_index + 1) % pattern.len();
        }
        
        data
    }
}

/// 格式化工具
pub struct Formatter;

impl Formatter {
    /// 格式化字节大小
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
    
    /// 格式化持续时间
    pub fn format_duration(duration: Duration) -> String {
        let nanos = duration.as_nanos();
        
        if nanos < 1_000 {
            format!("{} ns", nanos)
        } else if nanos < 1_000_000 {
            format!("{:.2} μs", nanos as f64 / 1_000.0)
        } else if nanos < 1_000_000_000 {
            format!("{:.2} ms", nanos as f64 / 1_000_000.0)
        } else {
            format!("{:.2} s", nanos as f64 / 1_000_000_000.0)
        }
    }
    
    /// 格式化百分比
    pub fn format_percentage(value: f64, total: f64) -> String {
        if total == 0.0 {
            "0.00%".to_string()
        } else {
            format!("{:.2}%", (value / total) * 100.0)
        }
    }
}