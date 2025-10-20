use mi7soft::examples::*;
use mi7soft::utils::*;
use std::env;

#[tokio::main]
async fn main() -> mi7soft::Result<()> {
    println!("🚀 Rust 共享内存和锁机制 Demo");
    println!("=====================================\n");
    
    // 显示系统信息
    let system_info = SystemInfo::new();
    system_info.print_system_info();
    println!();
    
    // 创建内存监控器
    let mut memory_monitor = MemoryMonitor::new();
    
    // 检查命令行参数
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "multithreaded" => {
                println!("运行多线程示例...");
                multithreaded_shared_memory_example()?;
            }
            "performance" => {
                println!("运行性能比较示例...");
                lock_performance_comparison()?;
            }
            "rwlock" => {
                println!("运行读写锁示例...");
                rwlock_example()?;
            }
            "rayon" => {
                println!("运行Rayon并行示例...");
                rayon_parallel_example()?;
            }
            "tokio" => {
                println!("运行Tokio异步示例...");
                tokio_async_example().await?;
            }
            "cross-process" => {
                println!("运行跨进程示例...");
                cross_process_example()?;
            }
            "all" => {
                println!("运行所有示例...");
                run_all_examples().await?;
            }
            _ => {
                print_usage();
                return Ok(());
            }
        }
    } else {
        // 默认运行所有示例
        run_all_examples().await?;
    }
    
    // 显示内存使用变化
    println!("\n📊 内存使用情况:");
    memory_monitor.print_memory_delta();
    
    println!("\n✅ Demo 运行完成！");
    Ok(())
}

fn print_usage() {
    println!("用法: cargo run [示例类型]");
    println!();
    println!("可用的示例类型:");
    println!("  multithreaded  - 多线程共享内存示例");
    println!("  performance    - 锁性能比较示例");
    println!("  rwlock         - 读写锁示例");
    println!("  rayon          - Rayon并行计算示例");
    println!("  tokio          - Tokio异步示例");
    println!("  cross-process  - 跨进程共享内存示例");
    println!("  all            - 运行所有示例（默认）");
    println!();
    println!("示例:");
    println!("  cargo run");
    println!("  cargo run performance");
    println!("  cargo run cross-process");
}
