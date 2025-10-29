use mi7::config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化配置系统
    config::init_config()?;

    println!("=== MI7 配置读取测试 ===");

    // 测试字符串配置读取
    println!("\n--- 字符串配置 ---");
    println!(
        "shared_memory.name: {}",
        config::string("shared_memory", "name")
    );
    println!(
        "logging.file_prefix: {}",
        config::string("logging", "file_prefix")
    );
    println!("logging.level: {}", config::string("logging", "level"));
    println!(
        "http.bind_address: {}",
        config::string("http", "bind_address")
    );
    println!("queue.name: {}", config::string("queue", "name"));

    // 测试整数配置读取
    println!("\n--- 整数配置 ---");
    println!(
        "shared_memory.slot_size: {}",
        config::int("shared_memory", "slot_size")
    );
    println!(
        "shared_memory.slot_count: {}",
        config::int("shared_memory", "slot_count")
    );
    println!("http.port: {}", config::int("http", "port"));
    println!(
        "http.timeout_seconds: {}",
        config::int("http", "timeout_seconds")
    );
    println!(
        "http.max_connections: {}",
        config::int("http", "max_connections")
    );
    println!("queue.capacity: {}", config::int("queue", "capacity"));

    // 测试布尔配置读取
    println!("\n--- 布尔配置 ---");
    println!(
        "logging.console_output: {}",
        config::bool("logging", "console_output")
    );
    println!("queue.persistent: {}", config::bool("queue", "persistent"));

    println!("\n=== 配置读取测试完成 ===");

    Ok(())
}
