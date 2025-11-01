use anyhow::Result;
use mi7::{BoxConfig, BoxSize, BoxState, SharedMailbox};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    println!("🚀 开始测试共享内存寄存箱功能");

    // 创建自定义配置
    let mut config = BoxConfig::new();
    config
        .set_count(BoxSize::Size1M, 10) // 10个 1MB box
        .set_count(BoxSize::Size2M, 5) // 5个 2MB box
        .set_count(BoxSize::Size5M, 2) // 2个 5MB box
        .set_count(BoxSize::Size10M, 1) // 1个 10MB box
        .set_count(BoxSize::Size20M, 1) // 1个 20MB box
        .set_count(BoxSize::Size50M, 1); // 1个 50MB box

    // 创建共享内存寄存箱
    let mailbox = Arc::new(SharedMailbox::new(config)?);
    println!("✅ 成功创建共享内存寄存箱");

    // 显示初始统计信息
    let stats: mi7::MailboxStats = mailbox.get_stats();
    println!("📊 初始统计信息:");
    println!("   总 box 数量: {}", stats.total_count);
    println!("   空 box 数量: {}", stats.empty_count);
    println!("   各大小 box 数量:");
    for (size, count) in &stats.size_counts {
        println!("     {:?}: {} 个", size, count);
    }

    // 测试基本的写入和读取流程
    test_basic_workflow(&*mailbox)?;

    // 测试多线程并发访问
    test_concurrent_access(&mailbox)?;

    // 测试不同大小的 box
    test_different_sizes(&*mailbox)?;

    println!("🎉 所有测试完成！");
    Ok(())
}

/// 测试基本的写入和读取流程
fn test_basic_workflow(mailbox: &SharedMailbox) -> Result<()> {
    println!("\n🧪 测试基本工作流程");

    // 步骤 1: A进程锁定整个内存空间
    println!("1. 获取全局锁...");
    let _lock = mailbox.lock()?;
    println!("   ✅ 成功获取全局锁");

    // 步骤 2: A进程获取一个空的box
    println!("2. 获取空的 1M box...");
    let box_id = mailbox.get_empty_box(BoxSize::Size1M)?;
    println!("   ✅ 获取到 box ID: {}", box_id);

    // 验证 box 状态为写入中
    let metadata = mailbox.find_box_by_id(box_id)?;
    assert_eq!(metadata.get_state(), BoxState::Writing);
    println!("   ✅ box 状态已设置为写入中");

    // 步骤 3: A进程写入数据到box
    println!("3. 写入数据到 box...");
    let test_data = b"Hello, Shared Mailbox! This is a test message from process A.";
    mailbox.write_data(box_id, test_data)?;
    println!("   ✅ 成功写入 {} 字节数据", test_data.len());

    // 验证 box 状态为满
    assert_eq!(metadata.get_state(), BoxState::Full);
    println!("   ✅ box 状态已设置为满");

    // 步骤 4: B进程将对应的box置为读取中
    println!("4. 设置 box 为读取中...");
    mailbox.start_reading(box_id)?;
    assert_eq!(metadata.get_state(), BoxState::Reading);
    println!("   ✅ box 状态已设置为读取中");

    // 步骤 5: B进程读取数据
    println!("5. 读取数据...");
    let read_data = mailbox.read_data(box_id)?;
    println!("   ✅ 成功读取 {} 字节数据", read_data.len());
    assert_eq!(read_data, test_data);
    println!("   ✅ 数据验证成功");

    // 步骤 6: B进程完成读取，将box置为空
    println!("6. 完成读取，释放 box...");
    mailbox.finish_reading(box_id)?;
    assert_eq!(metadata.get_state(), BoxState::Empty);
    println!("   ✅ box 状态已设置为空");

    println!("✅ 基本工作流程测试完成");
    Ok(())
}

/// 测试多线程并发访问
fn test_concurrent_access(mailbox: &Arc<SharedMailbox>) -> Result<()> {
    println!("\n🧪 测试多线程并发访问");

    let mut handles = vec![];

    // 创建多个写入线程
    for i in 0..3 {
        let mailbox_clone = Arc::clone(mailbox);
        let handle = thread::spawn(move || -> Result<()> {
            // 获取锁
            let _lock = mailbox_clone.lock()?;

            // 获取 box
            let box_id = mailbox_clone.get_empty_box(BoxSize::Size2M)?;
            println!("   线程 {} 获取到 box ID: {}", i, box_id);

            // 写入数据
            let data = format!("Thread {} data: {}", i, "x".repeat(1000));
            mailbox_clone.write_data(box_id, data.as_bytes())?;
            println!("   线程 {} 写入完成", i);

            // 模拟一些处理时间
            thread::sleep(Duration::from_millis(50));

            // 读取数据
            mailbox_clone.start_reading(box_id)?;
            let read_data = mailbox_clone.read_data(box_id)?;
            assert_eq!(read_data, data.as_bytes());

            // 完成读取
            mailbox_clone.finish_reading(box_id)?;
            println!("   线程 {} 完成", i);

            Ok(())
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap()?;
    }

    println!("✅ 多线程并发访问测试完成");
    Ok(())
}

/// 测试不同大小的 box
fn test_different_sizes(mailbox: &SharedMailbox) -> Result<()> {
    println!("\n🧪 测试不同大小的 box");

    let sizes = vec![BoxSize::Size1M, BoxSize::Size5M, BoxSize::Size10M];

    for size in sizes {
        println!("测试 {:?} box...", size);

        let _lock = mailbox.lock()?;
        let box_id = mailbox.get_empty_box(size)?;

        // 创建测试数据（不超过 box 大小）
        let data_size = std::cmp::min(size.bytes() / 2, 1024 * 1024); // 最多 1MB 测试数据
        let test_data = vec![0xAB; data_size];

        // 写入和读取
        mailbox.write_data(box_id, &test_data)?;
        mailbox.start_reading(box_id)?;
        let read_data = mailbox.read_data(box_id)?;
        mailbox.finish_reading(box_id)?;

        assert_eq!(read_data, test_data);
        println!("   ✅ {:?} box 测试成功 ({} 字节)", size, data_size);
    }

    println!("✅ 不同大小 box 测试完成");
    Ok(())
}

/// 测试错误情况
#[allow(dead_code)]
fn test_error_cases(mailbox: &SharedMailbox) -> Result<()> {
    println!("\n🧪 测试错误情况");

    let _lock = mailbox.lock()?;
    let box_id = mailbox.get_empty_box(BoxSize::Size1M)?;

    // 测试写入过大的数据
    let large_data = vec![0; BoxSize::Size1M.bytes() + 1];
    match mailbox.write_data(box_id, &large_data) {
        Err(_) => println!("   ✅ 正确拒绝过大数据"),
        Ok(_) => panic!("应该拒绝过大数据"),
    }

    // 测试在错误状态下读取
    match mailbox.read_data(box_id) {
        Err(_) => println!("   ✅ 正确拒绝在写入状态下读取"),
        Ok(_) => panic!("应该拒绝在写入状态下读取"),
    }

    println!("✅ 错误情况测试完成");
    Ok(())
}

/// 显示详细的统计信息
#[allow(dead_code)]
fn show_detailed_stats(mailbox: &SharedMailbox) {
    let stats = mailbox.get_stats();
    println!("\n📊 详细统计信息:");
    println!("   总数: {}", stats.total_count);
    println!("   空: {}", stats.empty_count);
    println!("   写入中: {}", stats.writing_count);
    println!("   满: {}", stats.full_count);
    println!("   读取中: {}", stats.reading_count);
    println!("   各大小分布:");
    for (size, count) in &stats.size_counts {
        println!("     {:?}: {}", size, count);
    }
}
