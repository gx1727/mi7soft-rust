use mi7::async_futex::AsyncFutex;
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
use tokio::time::sleep;

/// AsyncFutex 使用示例
///
/// 这个示例展示了如何使用 AsyncFutex 进行异步同步操作。
/// AsyncFutex 是基于 Linux futex 系统调用的异步同步原语，
/// 主要用于跨进程同步场景。
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 AsyncFutex 异步同步原语示例");
    println!("================================");
    println!("注意：AsyncFutex 主要用于跨进程同步");
    println!("在实际应用中，共享内存会在进程间共享");

    // 示例1: 基本 API 使用
    basic_api_example().await?;
    //
    // // 示例2: 状态变化演示
    // state_change_example().await?;

    // 示例3: 唤醒机制演示
    // wake_mechanism_example().await?;

    println!("\n✅ 所有示例执行完成！");
    println!("\n📚 AsyncFutex 关键特性：");
    println!("   • 基于 Linux futex 系统调用");
    println!("   • 结合 eventfd 实现异步通知");
    println!("   • 适用于跨进程同步场景");
    println!("   • 支持异步等待和唤醒操作");
    Ok(())
}

/// 示例1: 基本 API 使用
async fn basic_api_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 示例1: 基本 API 使用");
    println!("----------------------");

    // 创建共享的原子变量（在实际应用中，这会是共享内存中的变量）
    let shared_value: &mut AtomicU32 = Box::leak(Box::new(AtomicU32::new(0)));
    let futex = AsyncFutex::new(shared_value)?;

    println!("✅ 成功创建 AsyncFutex");
    println!("   初始值: {}", shared_value.load(Ordering::SeqCst));
    println!("   共享内存地址: {:p}", shared_value);

    // 演示基本操作
    println!("\n🔧 基本操作演示：");

    // 改变值
    shared_value.store(42, Ordering::SeqCst);
    println!("   设置值为 42");

    futex.wait_async(1).await()?;

    // 发送唤醒信号
    futex.wake(1);
    println!("   发送唤醒信号 (wake 1 个等待者)");

    // 再次改变值
    shared_value.store(100, Ordering::SeqCst);
    println!("   设置值为 100");

    // 发送广播唤醒
    futex.wake(u32::MAX);
    println!("   发送广播唤醒信号 (wake 所有等待者)");

    println!("   最终值: {}", shared_value.load(Ordering::SeqCst));
    Ok(())
}

/// 示例2: 状态变化演示
async fn state_change_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 示例2: 状态变化演示");
    println!("----------------------");

    // 创建状态变量
    let state = Box::leak(Box::new(AtomicU32::new(0)));
    let futex = AsyncFutex::new(state)?;

    println!("模拟应用程序状态变化：");

    // 定义状态转换
    let states = vec![
        (1, "🚀 应用启动"),
        (2, "⚙️ 加载配置"),
        (3, "🔗 建立连接"),
        (4, "📊 初始化数据"),
        (5, "✅ 就绪状态"),
    ];

    for (state_value, description) in states {
        // 模拟状态处理时间
        sleep(Duration::from_millis(200)).await;

        let old_state = state.swap(state_value, Ordering::SeqCst);
        println!(
            "   状态变化: {} -> {} ({})",
            old_state, state_value, description
        );

        // 通知状态变化
        futex.wake(1);
        println!("   📢 发送状态变化通知");
    }

    println!("   🎯 最终状态: {}", state.load(Ordering::SeqCst));
    Ok(())
}

/// 示例3: 唤醒机制演示
async fn wake_mechanism_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 示例3: 唤醒机制演示");
    println!("----------------------");

    // 创建计数器
    let counter = Box::leak(Box::new(AtomicU32::new(0)));
    let futex = AsyncFutex::new(counter)?;

    println!("演示不同的唤醒策略：");

    // 单个唤醒
    println!("\n🔔 单个唤醒演示：");
    for i in 1..=3 {
        counter.fetch_add(1, Ordering::SeqCst);
        futex.wake(1); // 只唤醒一个等待者
        println!("   计数: {}, 唤醒 1 个等待者", i);
        sleep(Duration::from_millis(100)).await;
    }

    // 批量唤醒
    println!("\n📢 批量唤醒演示：");
    for i in 1..=2 {
        counter.fetch_add(5, Ordering::SeqCst);
        futex.wake(5); // 唤醒 5 个等待者
        println!("   计数增加 5, 唤醒 5 个等待者");
        sleep(Duration::from_millis(100)).await;
    }

    // 广播唤醒
    println!("\n📻 广播唤醒演示：");
    counter.store(999, Ordering::SeqCst);
    futex.wake(u32::MAX); // 唤醒所有等待者
    println!("   设置特殊值 999, 广播唤醒所有等待者");

    println!("   🏁 最终计数: {}", counter.load(Ordering::SeqCst));
    Ok(())
}
