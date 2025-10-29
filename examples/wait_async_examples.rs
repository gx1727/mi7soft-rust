use std::sync::atomic::{AtomicU32, Ordering};
use mi7::async_futex::AsyncFutex;
use tokio::time::{timeout, Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 AsyncFutex wait_async 使用示例");
    println!("=====================================");
    println!("📚 wait_async 方法说明:");
    println!("   - 如果当前值不等于期望值，立即返回");
    println!("   - 如果当前值等于期望值，等待唤醒信号");
    println!("   - 可以配合 timeout 使用来避免无限等待");
    println!();

    // 示例1: 值不匹配时立即返回
    immediate_return_example().await?;

    // // 示例2: 超时机制
    // timeout_example().await?;
    //
    // // 示例3: 手动唤醒（使用 select!）
    // manual_wake_example().await?;

    println!("\n✅ 所有示例执行完成！");
    println!("\n📖 总结:");
    println!("   1. wait_async(expected) 会检查当前值是否等于 expected");
    println!("   2. 如果不等于，立即返回");
    println!("   3. 如果等于，等待其他任务调用 wake() 来唤醒");
    println!("   4. 建议总是使用 timeout 来避免无限等待");
    println!("   5. 在实际应用中，通常在多进程或多任务环境中使用");

    Ok(())
}

/// 示例1: 值不匹配时立即返回
async fn immediate_return_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 示例1: 值不匹配时立即返回");
    println!("---------------------------");

    let shared_value = Box::leak(Box::new(AtomicU32::new(42)));
    let futex = AsyncFutex::new(shared_value)?;

    println!("🔍 当前值: {}", shared_value.load(Ordering::SeqCst));
    println!("⏳ 等待值 0（与当前值 42 不匹配）...");
    
    let start = std::time::Instant::now();
    futex.wait_async(42).await?;
    let elapsed = start.elapsed();
    
    println!("✅ wait_async 立即返回！耗时: {:?}", elapsed);
    println!("💡 因为当前值 ({}) ≠ 期望值 (0)", shared_value.load(Ordering::SeqCst));

    Ok(())
}

/// 示例2: 超时机制
async fn timeout_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 示例2: 超时机制");
    println!("------------------");

    let shared_value = Box::leak(Box::new(AtomicU32::new(100)));
    let futex = AsyncFutex::new(shared_value)?;

    println!("🔍 当前值: {}", shared_value.load(Ordering::SeqCst));
    println!("⏰ 等待值 100（匹配当前值），设置 500ms 超时...");
    
    let start = std::time::Instant::now();
    match timeout(Duration::from_millis(500), futex.wait_async(100)).await {
        Ok(result) => {
            result?;
            println!("✅ 等待成功完成");
        }
        Err(_) => {
            let elapsed = start.elapsed();
            println!("⏰ 等待超时，耗时: {:?}", elapsed);
            println!("💡 这是预期的，因为值匹配但没有唤醒信号");
        }
    }

    // 现在改变值，再次测试
    println!("\n🔄 改变值为 200，再次等待值 100...");
    shared_value.store(200, Ordering::SeqCst);
    
    let start = std::time::Instant::now();
    futex.wait_async(100).await?;
    let elapsed = start.elapsed();
    println!("✅ 立即返回，耗时: {:?}", elapsed);
    println!("💡 因为当前值 ({}) ≠ 期望值 (100)", shared_value.load(Ordering::SeqCst));

    Ok(())
}

/// 示例3: 手动唤醒（使用 select!）
async fn manual_wake_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 示例3: 手动唤醒演示");
    println!("----------------------");

    let shared_value = Box::leak(Box::new(AtomicU32::new(42)));
    let futex = AsyncFutex::new(shared_value)?;

    println!("� 当前值: {}", shared_value.load(Ordering::SeqCst));
    println!("� 演示场景:");
    println!("   1. 等待值 42（匹配当前值）");
    println!("   2. 1秒后发送唤醒信号");
    println!("   3. wait_async 应该被唤醒");

    let start = std::time::Instant::now();
    
    // 使用 tokio::select! 来同时处理等待和唤醒
    tokio::select! {
        result = futex.wait_async(42) => {
            match result {
                Ok(()) => {
                    let elapsed = start.elapsed();
                    println!("✅ wait_async 被唤醒！耗时: {:?}", elapsed);
                }
                Err(e) => {
                    println!("❌ wait_async 出错: {}", e);
                }
            }
        }
        _ = async {
            sleep(Duration::from_millis(1000)).await;
            println!("   📡 发送唤醒信号...");
            futex.wake(1);
            // 让 select! 继续等待 wait_async 完成
            sleep(Duration::from_millis(100)).await;
        } => {
            println!("⏰ 唤醒任务完成");
        }
    }

    println!("🎉 唤醒演示完成！");

    Ok(())
}

/// 演示错误的用法（会导致死锁）
#[allow(dead_code)]
async fn deadlock_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚠️  错误示例: 可能导致死锁的用法");
    println!("--------------------------------");

    let shared_value = Box::leak(Box::new(AtomicU32::new(42)));
    let futex = AsyncFutex::new(shared_value)?;

    println!("🔍 当前值: {}", shared_value.load(Ordering::SeqCst));
    println!("❌ 错误: 等待值 42 但没有设置超时或唤醒机制");
    println!("   这会导致程序永远等待...");
    
    // 这是错误的用法 - 会导致死锁
    // futex.wait_async(42).await?;
    
    println!("💡 正确做法: 总是使用 timeout 或确保有唤醒机制");

    Ok(())
}

/// 实际应用场景示例
#[allow(dead_code)]
async fn practical_usage_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 实际应用场景示例");
    println!("--------------------");

    // 模拟一个状态机
    const STATE_IDLE: u32 = 0;
    const STATE_PROCESSING: u32 = 1;
    const STATE_DONE: u32 = 2;

    let state = Box::leak(Box::new(AtomicU32::new(STATE_IDLE)));
    let futex = AsyncFutex::new(state)?;

    println!("� 状态机示例:");
    println!("   IDLE = {}, PROCESSING = {}, DONE = {}", STATE_IDLE, STATE_PROCESSING, STATE_DONE);
    println!("   当前状态: {}", state.load(Ordering::SeqCst));

    // 等待状态变化（带超时）
    println!("\n⏳ 等待状态从 IDLE 变化...");
    match timeout(Duration::from_millis(500), futex.wait_async(STATE_IDLE)).await {
        Ok(result) => {
            result?;
            println!("✅ 状态已变化");
        }
        Err(_) => {
            println!("⏰ 超时 - 状态仍为 IDLE");
        }
    }

    // 模拟状态变化
    state.store(STATE_PROCESSING, Ordering::SeqCst);
    println!("� 状态变更为: PROCESSING");

    // 现在等待 IDLE 状态应该立即返回
    futex.wait_async(STATE_IDLE).await?;
    println!("✅ 检测到状态不再是 IDLE");

    Ok(())
}