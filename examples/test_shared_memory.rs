use mi7::{SharedMemoryMailbox, BoxConfig, BoxSize};
use std::env;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage: {} <writer|reader>", args[0]);
        return Ok(());
    }

    let mode = &args[1];
    let shared_file = "mailbox_shared.dat";

    // 创建配置
    let mut config = BoxConfig::new();
    config.set_count(BoxSize::Size1M, 5);
    config.set_count(BoxSize::Size2M, 3);
    config.set_count(BoxSize::Size5M, 2);

    match mode.as_str() {
        "writer" => run_writer(shared_file, config),
        "reader" => run_reader(shared_file, config),
        _ => {
            println!("Invalid mode. Use 'writer' or 'reader'");
            Ok(())
        }
    }
}

fn run_writer(shared_file: &str, config: BoxConfig) -> anyhow::Result<()> {
    println!("启动写进程...");
    
    // 创建或打开共享内存邮箱
    let mailbox = SharedMemoryMailbox::new_shared(shared_file, config.clone())?;
    
    println!("共享内存邮箱创建成功");
    println!("初始统计: {:?}", mailbox.get_stats());

    // 写入测试数据
    for i in 0..10 {
        let _lock = mailbox.lock()?;
        
        // 获取一个1MB的空box
        match mailbox.get_empty_box(BoxSize::Size1M) {
            Ok(box_id) => {
                let data = format!("Message {} from writer process", i);
                mailbox.write_data(box_id, data.as_bytes())?;
                println!("写入消息 {}: box_id={}, data='{}'", i, box_id, data);
            }
            Err(e) => {
                println!("无法获取空box: {}", e);
            }
        }
        
        thread::sleep(Duration::from_millis(500));
    }

    println!("写进程完成，最终统计: {:?}", mailbox.get_stats());
    Ok(())
}

fn run_reader(shared_file: &str, config: BoxConfig) -> anyhow::Result<()> {
    println!("启动读进程...");
    
    // 等待一下确保写进程已经创建了共享内存
    thread::sleep(Duration::from_millis(100));
    
    // 打开现有的共享内存邮箱
    let mailbox = SharedMemoryMailbox::new_shared(shared_file, config)?;
    
    println!("连接到共享内存邮箱成功");
    println!("初始统计: {:?}", mailbox.get_stats());

    let mut messages_read = 0;
    let max_attempts = 50; // 最多尝试50次
    
    for attempt in 0..max_attempts {
        let _lock = mailbox.lock()?;
        
        // 查找所有满的box
        let stats = mailbox.get_stats();
        let full_boxes = stats.total_count - stats.empty_count;
        
        if full_boxes > 0 {
            // 尝试读取数据（这里简化处理，实际应用中需要更复杂的逻辑来找到满的box）
            for box_id in 1..=stats.total_count as u32 {
                if let Ok(()) = mailbox.start_reading(box_id) {
                    match mailbox.read_data(box_id) {
                        Ok(data) => {
                            let message = String::from_utf8_lossy(&data);
                            println!("读取消息: box_id={}, data='{}'", box_id, message);
                            mailbox.finish_reading(box_id)?;
                            messages_read += 1;
                        }
                        Err(e) => {
                            println!("读取数据失败: {}", e);
                        }
                    }
                    break; // 每次只读一个消息
                }
            }
        }
        
        if messages_read >= 10 {
            break;
        }
        
        thread::sleep(Duration::from_millis(200));
    }

    println!("读进程完成，共读取 {} 条消息", messages_read);
    println!("最终统计: {:?}", mailbox.get_stats());
    Ok(())
}