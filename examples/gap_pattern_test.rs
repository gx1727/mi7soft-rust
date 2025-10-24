use mi7::{SharedRingQueue, Message};
use std::sync::{Arc, Mutex};

// 安全包装器
struct SafeSharedRing {
    ptr: *mut SharedRingQueue<4, 1024>,
}

unsafe impl Send for SafeSharedRing {}
unsafe impl Sync for SafeSharedRing {}

impl SafeSharedRing {
    unsafe fn new() -> Self {
        let ptr = unsafe { SharedRingQueue::open("/test_gap_pattern", true) };
        Self { ptr }
    }

    unsafe fn push<T: bincode::Encode>(&mut self, value: &T) -> Result<u64, &'static str> {
        unsafe { (*self.ptr).push(value) }
    }

    unsafe fn pop<T: bincode::Decode<()>>(&mut self) -> Option<(u64, T)> {
        unsafe { (*self.ptr).pop::<T>() }
    }
}

fn main() {
    println!("🧪 测试是否可能出现 [FULL, EMPTY, EMPTY, FULL] 状态");
    
    let queue = Arc::new(Mutex::new(unsafe { SafeSharedRing::new() }));
    
    // 尝试构造 [FULL, EMPTY, EMPTY, FULL] 状态
    println!("\n📝 尝试构造特定状态...");
    
    // 步骤1: 写入4个消息，填满队列
    println!("\n🔸 步骤1: 填满队列");
    for i in 1..=4 {
        let message = Message {
            id: i,
            data: format!("消息 {}", i).into_bytes(),
            timestamp: 0,
        };
        
        let result = {
            let mut queue_guard = queue.lock().unwrap();
            unsafe { queue_guard.push(&message) }
        };
        
        match result {
            Ok(request_id) => println!("✅ 消息 {} 写入成功，ID: {}", i, request_id),
            Err(e) => println!("❌ 消息 {} 写入失败: {}", i, e),
        }
    }
    // 现在状态: [FULL, FULL, FULL, FULL], head=0, tail=0
    
    // 步骤2: 消费位置0的消息
    println!("\n🔸 步骤2: 消费位置0的消息");
    {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
        }
    }
    // 现在状态: [EMPTY, FULL, FULL, FULL], head=1, tail=0
    
    // 步骤3: 写入一个新消息到位置0
    println!("\n🔸 步骤3: 写入新消息到位置0");
    let message = Message {
        id: 5,
        data: "新消息5".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => println!("✅ 新消息写入成功，ID: {}", request_id),
        Err(e) => println!("❌ 新消息写入失败: {}", e),
    }
    // 现在状态: [FULL, FULL, FULL, FULL], head=1, tail=1
    
    // 步骤4: 消费位置1的消息
    println!("\n🔸 步骤4: 消费位置1的消息");
    {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
        }
    }
    // 现在状态: [FULL, EMPTY, FULL, FULL], head=2, tail=1
    
    // 步骤5: 消费位置2的消息
    println!("\n🔸 步骤5: 消费位置2的消息");
    {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
        }
    }
    // 现在状态: [FULL, EMPTY, EMPTY, FULL], head=3, tail=1
    
    println!("\n🎯 理论上现在应该是 [FULL, EMPTY, EMPTY, FULL] 状态！");
    
    // 步骤6: 验证状态 - 尝试写入应该失败（因为tail=1，位置1是EMPTY，应该能写入）
    println!("\n🔸 步骤6: 验证 - 尝试写入新消息");
    let message = Message {
        id: 6,
        data: "验证消息6".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 验证消息写入成功，ID: {} (写入到位置1)", request_id);
            println!("📊 这证明了 [FULL, EMPTY, EMPTY, FULL] 状态是可能的！");
        }
        Err(e) => {
            println!("❌ 验证消息写入失败: {}", e);
            println!("📊 这说明当前状态不是我们预期的");
        }
    }
    
    // 步骤7: 继续验证 - 再次尝试写入
    println!("\n🔸 步骤7: 再次尝试写入");
    let message = Message {
        id: 7,
        data: "验证消息7".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 第二个验证消息写入成功，ID: {} (写入到位置2)", request_id);
        }
        Err(e) => {
            println!("❌ 第二个验证消息写入失败: {}", e);
        }
    }
    
    // 步骤8: 第三次尝试写入
    println!("\n🔸 步骤8: 第三次尝试写入");
    let message = Message {
        id: 8,
        data: "验证消息8".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 第三个验证消息写入成功，ID: {}", request_id);
        }
        Err(e) => {
            println!("❌ 第三个验证消息写入失败: {} (这里应该失败，因为队列满了)", e);
        }
    }
    
    // 步骤9: 消费剩余消息
    println!("\n📊 最终验证: 消费所有剩余消息");
    let mut count = 0;
    loop {
        let result = {
            let mut queue_guard = queue.lock().unwrap();
            unsafe { queue_guard.pop::<Message>() }
        };
        
        match result {
            Some((request_id, message)) => {
                let content = String::from_utf8_lossy(&message.data);
                println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
                count += 1;
            }
            None => {
                println!("📭 队列已空，总共消费了 {} 条消息", count);
                break;
            }
        }
    }
    
    println!("\n✅ 测试完成");
}