use mi7::config;

fn main() {
    println!("=== 新配置系统使用示例 ===");
    
    // 初始化配置系统
    let _ = config::init_config();
    
    // 方式1: 使用传统的 section.key 方式访问
    println!("\n1. 传统方式访问配置:");
    let interface_name = config::string("worker", "interface_name");
    let log_level = config::string("worker", "log_level");
    let queue_capacity = config::int("queue", "capacity");
    
    println!("worker.interface_name: {}", interface_name);
    println!("worker.log_level: {}", log_level);
    println!("queue.capacity: {}", queue_capacity);
    
    // 方式2: 使用 ConfigAccessor 简化访问
    println!("\n2. 简化方式访问配置:");
    let config_accessor = &config::CONFIG_ACCESSOR;
    
    // 默认在 worker 段中查找
    let interface_name2 = config_accessor.string("interface_name");
    let log_level2 = config_accessor.string("log_level");
    
    // 指定段名
    let shared_memory_name = config_accessor.string("shared_memory.name");
    let http_port = config_accessor.int("http.port");
    
    println!("interface_name (默认 worker 段): {}", interface_name2);
    println!("log_level (默认 worker 段): {}", log_level2);
    println!("shared_memory.name: {}", shared_memory_name);
    println!("http.port: {}", http_port);
    
    // 方式3: 使用带默认值的访问（推荐用于动态配置）
    println!("\n3. 带默认值的配置访问（支持动态扩展）:");
    
    // 这些配置项可能不存在，但会返回默认值
    let hello_value = config_accessor.string_or("hello", "world");
    let custom_port = config_accessor.int_or("custom_port", 9999);
    let debug_mode = config_accessor.bool_or("debug", false);
    let timeout = config_accessor.int_or("timeout", 30);
    
    println!("hello (默认: world): {}", hello_value);
    println!("custom_port (默认: 9999): {}", custom_port);
    println!("debug (默认: false): {}", debug_mode);
    println!("timeout (默认: 30): {}", timeout);
    
    println!("\n4. 演示动态配置扩展:");
    
    // 获取配置实例
    let mut config_instance = config::get_config().clone();
    
    // 动态添加新的配置项到 worker 段
    config_instance.set("worker", "hello", config::ConfigValue::String("world".to_string()));
    config_instance.set("worker", "custom_port", config::ConfigValue::Integer(9999));
    config_instance.set("worker", "debug", config::ConfigValue::Boolean(true));
    config_instance.set("worker", "timeout", config::ConfigValue::Integer(60));
    
    // 添加新的配置段
    config_instance.set("my_app", "version", config::ConfigValue::String("1.0.0".to_string()));
    config_instance.set("my_app", "max_users", config::ConfigValue::Integer(1000));
    config_instance.set("my_app", "enable_cache", config::ConfigValue::Boolean(true));
    
    // 保存到新文件
    if let Err(e) = config_instance.save_to_file("extended_config.toml") {
        eprintln!("保存配置失败: {}", e);
    } else {
        println!("扩展配置已保存到 extended_config.toml");
        
        // 从新文件加载并验证
        match config::Config::load_from_file("extended_config.toml") {
            Ok(loaded_config) => {
                println!("成功加载扩展配置");
                
                // 验证新添加的配置项
                if let Some(version) = loaded_config.get("my_app", "version") {
                    if let Some(version_str) = version.as_string() {
                        println!("my_app.version: {}", version_str);
                    }
                }
                
                if let Some(max_users) = loaded_config.get("my_app", "max_users") {
                    if let Some(max_users_int) = max_users.as_int() {
                        println!("my_app.max_users: {}", max_users_int);
                    }
                }
                
                if let Some(enable_cache) = loaded_config.get("my_app", "enable_cache") {
                    if let Some(enable_cache_bool) = enable_cache.as_bool() {
                        println!("my_app.enable_cache: {}", enable_cache_bool);
                    }
                }
            }
            Err(e) => {
                eprintln!("加载扩展配置失败: {}", e);
            }
        }
    }
    
    println!("\n=== 使用示例完成 ===");
    println!("\n总结:");
    println!("1. 新的配置系统支持动态添加配置项，无需修改结构体定义");
    println!("2. 支持多种访问方式：传统的 section.key 和简化的 key 访问");
    println!("3. 提供带默认值的访问方法，适合处理可选配置");
    println!("4. 完全兼容 TOML 格式，支持配置的保存和加载");
    println!("5. 类型安全，支持 String、Integer、Float、Boolean 类型");
}