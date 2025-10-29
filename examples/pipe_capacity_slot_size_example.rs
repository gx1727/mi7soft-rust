use mi7::{CrossProcessPipe, PipeConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 CrossProcessPipe CAPACITY 和 SLOT_SIZE 参数传递示例");
    println!("=======================================================");

    // ========================================
    // 方式1: 编译时常量泛型参数（推荐方式）
    // ========================================
    println!("\n📝 方式1: 编译时常量泛型参数");
    println!("----------------------------------");

    // 小型管道：10个槽位，每个1KB
    let small_pipe = CrossProcessPipe::<10, 1024>::create("/small_pipe")?;
    println!("✅ 小型管道: 容量={}, 槽位大小={} bytes", 
             small_pipe.capacity(), small_pipe.slot_size());

    // 默认管道：100个槽位，每个4KB
    let default_pipe = CrossProcessPipe::<100, 4096>::create("/default_pipe")?;
    println!("✅ 默认管道: 容量={}, 槽位大小={} bytes", 
             default_pipe.capacity(), default_pipe.slot_size());

    // 大型管道：1000个槽位，每个8KB
    let large_pipe = CrossProcessPipe::<1000, 8192>::create("/large_pipe")?;
    println!("✅ 大型管道: 容量={}, 槽位大小={} bytes", 
             large_pipe.capacity(), large_pipe.slot_size());

    // 自定义管道：500个槽位，每个2KB
    let custom_pipe = CrossProcessPipe::<500, 2048>::create("/custom_pipe")?;
    println!("✅ 自定义管道: 容量={}, 槽位大小={} bytes", 
             custom_pipe.capacity(), custom_pipe.slot_size());

    // ========================================
    // 方式2: 使用类型别名简化代码
    // ========================================
    println!("\n📝 方式2: 使用类型别名简化代码");
    println!("----------------------------------");

    // 定义常用的类型别名
    type SmallPipe = CrossProcessPipe<10, 1024>;
    type DefaultPipe = CrossProcessPipe<100, 4096>;
    type LargePipe = CrossProcessPipe<1000, 8192>;
    type HighFreqPipe = CrossProcessPipe<500, 512>;   // 高频小消息
    type LowFreqPipe = CrossProcessPipe<20, 16384>;   // 低频大消息

    let small_alias = SmallPipe::create("/small_alias")?;
    println!("✅ 小型别名管道: 容量={}, 槽位大小={} bytes", 
             small_alias.capacity(), small_alias.slot_size());

    let high_freq = HighFreqPipe::create("/high_freq")?;
    println!("✅ 高频管道: 容量={}, 槽位大小={} bytes", 
             high_freq.capacity(), high_freq.slot_size());

    let low_freq = LowFreqPipe::create("/low_freq")?;
    println!("✅ 低频管道: 容量={}, 槽位大小={} bytes", 
             low_freq.capacity(), low_freq.slot_size());

    // ========================================
    // 方式3: 配置验证（编译时参数验证）
    // ========================================
    println!("\n📝 方式3: 配置验证");
    println!("----------------------------------");

    // 创建配置对象
    let config = PipeConfig::new(100, 4096);
    println!("📋 配置: 容量={}, 槽位大小={} bytes", config.capacity, config.slot_size);

    // 使用配置创建管道（会验证参数匹配）
    let validated_pipe = CrossProcessPipe::<100, 4096>::create_with_config(
        "/validated_pipe", 
        config
    )?;
    println!("✅ 验证管道: 容量={}, 槽位大小={} bytes", 
             validated_pipe.capacity(), validated_pipe.slot_size());

    // 演示配置不匹配的情况
    let wrong_config = PipeConfig::new(200, 8192);  // 与泛型参数不匹配
    match CrossProcessPipe::<100, 4096>::create_with_config("/wrong_pipe", wrong_config) {
        Ok(_) => println!("❌ 这不应该成功"),
        Err(e) => println!("✅ 配置验证失败（预期）: {}", e),
    }

    // ========================================
    // 方式4: 预定义配置常量
    // ========================================
    println!("\n📝 方式4: 预定义配置常量");
    println!("----------------------------------");

    // 定义常用配置常量
    const SMALL_CAPACITY: usize = 10;
    const SMALL_SLOT_SIZE: usize = 1024;
    
    const DEFAULT_CAPACITY: usize = 100;
    const DEFAULT_SLOT_SIZE: usize = 4096;
    
    const LARGE_CAPACITY: usize = 1000;
    const LARGE_SLOT_SIZE: usize = 8192;

    let const_pipe = CrossProcessPipe::<DEFAULT_CAPACITY, DEFAULT_SLOT_SIZE>::create("/const_pipe")?;
    println!("✅ 常量管道: 容量={}, 槽位大小={} bytes", 
             const_pipe.capacity(), const_pipe.slot_size());

    // ========================================
    // 方式5: 场景化配置选择
    // ========================================
    println!("\n📝 方式5: 场景化配置选择");
    println!("----------------------------------");

    // 控制信号管道 - 小容量，小消息
    type ControlPipe = CrossProcessPipe<20, 256>;
    let control = ControlPipe::create("/control")?;
    println!("🎛️  控制管道: 容量={}, 槽位大小={} bytes", 
             control.capacity(), control.slot_size());

    // 数据传输管道 - 中等容量，中等消息
    type DataPipe = CrossProcessPipe<100, 4096>;
    let data = DataPipe::create("/data")?;
    println!("📊 数据管道: 容量={}, 槽位大小={} bytes", 
             data.capacity(), data.slot_size());

    // 文件传输管道 - 小容量，大消息
    type FilePipe = CrossProcessPipe<10, 65536>;
    let file = FilePipe::create("/file")?;
    println!("📁 文件管道: 容量={}, 槽位大小={} bytes", 
             file.capacity(), file.slot_size());

    // 日志管道 - 大容量，小消息
    type LogPipe = CrossProcessPipe<1000, 512>;
    let log = LogPipe::create("/log")?;
    println!("📝 日志管道: 容量={}, 槽位大小={} bytes", 
             log.capacity(), log.slot_size());

    // ========================================
    // 内存使用计算
    // ========================================
    println!("\n📊 内存使用计算");
    println!("----------------------------------");

    fn calculate_memory_usage(capacity: usize, slot_size: usize) -> usize {
        capacity * slot_size + 1024  // 槽位数据 + 元数据开销估算
    }

    let small_memory = calculate_memory_usage(10, 1024);
    let default_memory = calculate_memory_usage(100, 4096);
    let large_memory = calculate_memory_usage(1000, 8192);

    println!("💾 小型管道内存使用: ~{} KB", small_memory / 1024);
    println!("💾 默认管道内存使用: ~{} KB", default_memory / 1024);
    println!("💾 大型管道内存使用: ~{} KB", large_memory / 1024);

    println!("\n✅ 所有示例执行完成！");
    println!("\n📚 关键要点:");
    println!("   1. CAPACITY 和 SLOT_SIZE 是编译时常量泛型参数");
    println!("   2. 必须在类型声明时指定具体数值");
    println!("   3. 可以使用类型别名简化重复的泛型参数");
    println!("   4. 配置对象主要用于运行时验证");
    println!("   5. 根据使用场景选择合适的容量和槽位大小");

    Ok(())
}

// ========================================
// 额外示例：函数中使用泛型参数
// ========================================

/// 创建指定配置的管道的泛型函数
fn create_pipe_with_params<const C: usize, const S: usize>(
    name: &str
) -> Result<CrossProcessPipe<C, S>, Box<dyn std::error::Error>> {
    println!("🔧 创建管道: 名称={}, 容量={}, 槽位大小={}", name, C, S);
    CrossProcessPipe::<C, S>::create(name)
}

/// 连接到指定配置的管道的泛型函数
fn connect_pipe_with_params<const C: usize, const S: usize>(
    name: &str
) -> Result<CrossProcessPipe<C, S>, Box<dyn std::error::Error>> {
    println!("🔗 连接管道: 名称={}, 容量={}, 槽位大小={}", name, C, S);
    CrossProcessPipe::<C, S>::connect(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_different_configurations() {
        // 测试不同配置的管道创建
        let _small = create_pipe_with_params::<10, 1024>("/test_small").unwrap();
        let _medium = create_pipe_with_params::<100, 4096>("/test_medium").unwrap();
        let _large = create_pipe_with_params::<1000, 8192>("/test_large").unwrap();
    }

    #[test]
    fn test_type_aliases() {
        type TestPipe = CrossProcessPipe<50, 2048>;
        let _pipe = TestPipe::create("/test_alias").unwrap();
        assert_eq!(_pipe.capacity(), 50);
        assert_eq!(_pipe.slot_size(), 2048);
    }
}