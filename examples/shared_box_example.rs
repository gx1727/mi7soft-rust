//! # SharedMemoryMailbox 示例
//!
//! 这个示例演示了如何使用 SharedMemoryMailbox 进行进程间通信。
//! SharedMemoryMailbox 是一个基于共享内存的邮箱系统，支持多种大小的消息盒子。
//!
//! ## 核心概念
//!
//! - **Box**: 消息容器，有不同的大小（1MB, 2MB, 5MB 等）
//! - **BoxState**: Box 的状态（Empty, Writing, Full, Reading）
//! - **MailboxLock**: 全局锁，确保操作的原子性
//! - **共享内存**: 进程间共享的内存区域，用于存储消息
//!
//! ## 使用方法
//!
//! 写入进程：
//! ```bash
//! cargo run --bin shared_box_example writer
//! ```
//!
//! 读取进程：
//! ```bash
//! cargo run --bin shared_box_example reader
//! ```

use anyhow::Result;
use mi7::{SharedMemoryMailbox, BoxConfig, BoxSize};
use std::env;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        println!("用法: {} [writer|reader]", args[0]);
        println!("  writer - 写入数据到共享内存");
        println!("  reader - 从共享内存读取数据");
        return Ok(());
    }

    match args[1].as_str() {
        "writer" => run_writer(),
        "reader" => run_reader(),
        _ => {
            println!("无效参数: {}，请使用 'writer' 或 'reader'", args[1]);
            Ok(())
        }
    }
}

/// 写入进程示例
fn run_writer() -> Result<()> {
    println!("🚀 启动写入进程...");
    
    // 创建 box 配置
    let mut config = BoxConfig::new();
    config
        .set_count(BoxSize::Size1M, 5)   // 5个 1MB 的 box
        .set_count(BoxSize::Size2M, 3)   // 3个 2MB 的 box
        .set_count(BoxSize::Size5M, 2);  // 2个 5MB 的 box

    // 创建或连接到共享内存邮箱
    let mailbox = SharedMemoryMailbox::new_shared("example_mailbox", config)?;
    println!("✅ 共享内存邮箱创建/连接成功");

    // 显示初始统计信息
    let initial_stats = mailbox.get_stats();
    println!("📊 初始统计: {:?}", initial_stats);

    // 写入一些示例数据
    let messages = vec![
        ("Hello from writer!", BoxSize::Size1M),
        ("This is a longer message that demonstrates the shared memory functionality.", BoxSize::Size1M),
        ("Medium sized message for 2MB box.", BoxSize::Size2M),
        ("Large message for 5MB box - this could contain much more data in a real application.", BoxSize::Size5M),
    ];

    for (i, (message, size)) in messages.iter().enumerate() {
        match write_message(&mailbox, message, *size, i + 1) {
            Ok(box_id) => println!("✅ 消息 {} 写入成功，box_id: {}", i + 1, box_id),
            Err(e) => println!("❌ 消息 {} 写入失败: {}", i + 1, e),
        }
        
        // 短暂延迟
        thread::sleep(Duration::from_millis(100));
    }

    // 显示最终统计信息
    let final_stats = mailbox.get_stats();
    println!("📊 写入完成后统计: {:?}", final_stats);
    
    println!("🎉 写入进程完成！");
    Ok(())
}

/// 读取进程示例
fn run_reader() -> Result<()> {
    println!("🚀 启动读取进程...");
    
    // 创建相同的 box 配置
    let mut config = BoxConfig::new();
    config
        .set_count(BoxSize::Size1M, 5)
        .set_count(BoxSize::Size2M, 3)
        .set_count(BoxSize::Size5M, 2);

    // 连接到已存在的共享内存邮箱
    let mailbox = SharedMemoryMailbox::new_shared("example_mailbox", config)?;
    println!("✅ 连接到共享内存邮箱成功");

    // 显示初始统计信息
    let initial_stats = mailbox.get_stats();
    println!("📊 初始统计: {:?}", initial_stats);

    // 读取所有可用的消息
    let mut read_count = 0;
    let max_attempts = 50; // 最多尝试50次

    for attempt in 0..max_attempts {
        let stats = mailbox.get_stats();
        if stats.full_count == 0 {
            if attempt > 10 {
                println!("📭 没有更多消息可读取");
                break;
            }
            thread::sleep(Duration::from_millis(100));
            continue;
        }

        // 获取所有满的 box
        let full_boxes = mailbox.get_full_boxes();
        if let Some(&box_id) = full_boxes.first() {
            match read_message(&mailbox, box_id) {
                Ok(data) => {
                    read_count += 1;
                    let message = String::from_utf8_lossy(&data);
                    println!("📨 读取消息 {}: box_id={}, 内容='{}'", read_count, box_id, message);
                }
                Err(e) => println!("❌ 读取 box {} 失败: {}", box_id, e),
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    // 显示最终统计信息
    let final_stats = mailbox.get_stats();
    println!("📊 读取完成后统计: {:?}", final_stats);
    println!("🎉 读取进程完成，共读取 {} 条消息", read_count);
    
    Ok(())
}

/// 写入消息到指定大小的 box
/// 
/// 这个函数演示了完整的消息写入流程：
/// 1. 获取全局锁以确保操作的原子性
/// 2. 获取一个空的 box 用于写入
/// 3. 写入消息数据到指定的 box
fn write_message(mailbox: &SharedMemoryMailbox, message: &str, size: BoxSize, msg_id: usize) -> Result<u32> {
    // 获取锁
    let _lock = mailbox.lock()?;
    
    // 获取空 box
    let box_id = mailbox.get_empty_box(size)?;
    
    // 写入数据
    let data = format!("Message {}: {}", msg_id, message);
    mailbox.write_data(box_id, data.as_bytes())?;
    
    Ok(box_id)
}

/// 从指定 box 读取消息
/// 
/// 这个函数演示了完整的消息读取流程：
/// 1. 获取全局锁以确保操作的原子性
/// 2. 开始读取操作（将 box 状态从 Full 设为 Reading）
/// 3. 读取消息数据
/// 4. 完成读取操作（将 box 状态设为 Empty，释放 box）
fn read_message(mailbox: &SharedMemoryMailbox, box_id: u32) -> Result<Vec<u8>> {
    // 获取锁
    let _lock = mailbox.lock()?;
    
    // 开始读取
    mailbox.start_reading(box_id)?;
    
    // 读取数据
    let data = mailbox.read_data(box_id)?;
    
    // 完成读取，释放 box
    mailbox.finish_reading(box_id)?;
    
    Ok(data)
}

