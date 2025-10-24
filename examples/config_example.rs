use mi7::config::{
    init_config, get_config, get_shared_memory_name, get_slot_size, 
    get_http_port, get_log_path, get_shared_memory_config,
    get_logging_config, get_http_config, get_queue_config,
    Config, ConfigError
};

fn main() -> Result<(), ConfigError> {
    println!("=== MI7 配置模块使用示例 ===\n");

    // 1. 初始化配置
    println!("1. 初始化配置...");
    init_config()?;
    println!("✓ 配置初始化成功\n");

    // 2. 获取完整配置
    println!("2. 完整配置信息:");
    let config = get_config();
    println!("{:#?}\n", config);

    // 3. 获取特定配置值
    println!("3. 常用配置值:");
    println!("   共享内存名称: {}", get_shared_memory_name());
    println!("   槽位大小: {} 字节", get_slot_size());
    println!("   HTTP端口: {}", get_http_port());
    println!("   日志路径: {:?}\n", get_log_path());

    // 4. 获取分类配置
    println!("4. 分类配置信息:");
    
    let shm_config = get_shared_memory_config();
    println!("   共享内存配置:");
    println!("     - 名称: {}", shm_config.name);
    println!("     - 槽位大小: {} 字节", shm_config.slot_size);
    println!("     - 槽位数量: {}", shm_config.slot_count);

    let log_config = get_logging_config();
    println!("   日志配置:");
    println!("     - 路径: {:?}", log_config.log_path);
    println!("     - 文件前缀: {}", log_config.file_prefix);
    println!("     - 控制台输出: {}", log_config.console_output);
    println!("     - 日志级别: {}", log_config.level);

    let http_config = get_http_config();
    println!("   HTTP配置:");
    println!("     - 端口: {}", http_config.port);
    println!("     - 绑定地址: {}", http_config.bind_address);
    println!("     - 超时时间: {} 秒", http_config.timeout_seconds);
    println!("     - 最大连接数: {}", http_config.max_connections);

    let queue_config = get_queue_config();
    println!("   队列配置:");
    println!("     - 容量: {}", queue_config.capacity);
    println!("     - 名称: {}", queue_config.name);
    println!("     - 持久化: {}\n", queue_config.persistent);

    // 5. 演示创建自定义配置
    println!("5. 创建自定义配置:");
    let mut custom_config = Config::default();
    custom_config.shared_memory.name = "custom_memory".to_string();
    custom_config.shared_memory.slot_size = 2048;
    custom_config.http.port = 9090;
    custom_config.logging.level = "debug".to_string();

    // 验证配置
    custom_config.validate()?;
    println!("✓ 自定义配置验证通过");

    // 保存到文件
    custom_config.save_to_file("custom_config.toml")?;
    println!("✓ 自定义配置已保存到 custom_config.toml");

    // 6. 从文件加载配置
    println!("\n6. 从文件加载配置:");
    let loaded_config = Config::load_from_file("custom_config.toml")?;
    println!("✓ 配置加载成功");
    println!("   加载的HTTP端口: {}", loaded_config.http.port);
    println!("   加载的共享内存名称: {}", loaded_config.shared_memory.name);

    println!("\n=== 配置模块示例完成 ===");
    Ok(())
}