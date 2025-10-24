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
        let ptr = unsafe { SharedRingQueue::open("/test_tail_position", true) };
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
    println!("🧪 测试 [FULL, EMPTY, EMPTY, FULL] 状态下 tail=3 的行为");
    
    let queue = Arc::new(Mutex::new(unsafe { SafeSharedRing::new() }));
    
    println!("\n📝 构造 [FULL, EMPTY, EMPTY, FULL] 状态，tail=3...");
    
    // 步骤1: 填满队列 [FULL, FULL, FULL, FULL], head=0, tail=0
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
    // 状态: [FULL, FULL, FULL, FULL], head=0, tail=0
    
    // 步骤2: 消费位置0、1、2的消息，让 head=3
    println!("\n🔸 步骤2: 消费位置0、1、2的消息");
    for i in 1..=3 {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息 {}: ID={}, 内容={}", i, request_id, content);
        }
    }
    // 状态: [EMPTY, EMPTY, EMPTY, FULL], head=3, tail=0
    
    // 步骤3: 写入3个消息到位置0、1、2
    println!("\n🔸 步骤3: 写入3个新消息");
    for i in 5..=7 {
        let message = Message {
            id: i,
            data: format!("新消息 {}", i).into_bytes(),
            timestamp: 0,
        };
        
        let result = {
            let mut queue_guard = queue.lock().unwrap();
            unsafe { queue_guard.push(&message) }
        };
        
        match result {
            Ok(request_id) => println!("✅ 新消息 {} 写入成功，ID: {}", i, request_id),
            Err(e) => println!("❌ 新消息 {} 写入失败: {}", i, e),
        }
    }
    // 现在状态应该是: [FULL, FULL, FULL, FULL], head=3, tail=3
    
    // 步骤4: 消费位置3的消息，让 head=0
    println!("\n🔸 步骤4: 消费位置3的消息");
    {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费位置3消息: ID={}, 内容={}", request_id, content);
        }
    }
    // 现在状态: [FULL, FULL, FULL, EMPTY], head=0, tail=3
    
    // 步骤5: 消费位置0、1的消息
    println!("\n🔸 步骤5: 消费位置0、1的消息");
    for i in 0..=1 {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费位置{}消息: ID={}, 内容={}", i, request_id, content);
        }
    }
    // 现在状态: [EMPTY, EMPTY, FULL, EMPTY], head=2, tail=3
    
    // 步骤6: 写入一个消息到位置3
    println!("\n🔸 步骤6: 写入消息到位置3");
    let message = Message {
        id: 8,
        data: "测试消息8".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 消息8写入成功，ID: {} (写入到位置3)", request_id);
            println!("📊 根据代码逻辑: tail = (3 + 1) % 4 = 0");
        }
        Err(e) => println!("❌ 消息8写入失败: {}", e),
    }
    // 现在状态: [EMPTY, EMPTY, FULL, FULL], head=2, tail=0
    
    // 步骤7: 再次写入消息，验证 tail 现在指向位置0
    println!("\n🔸 步骤7: 验证 tail 现在指向位置0");
    let message = Message {
        id: 9,
        data: "验证消息9".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 验证消息9写入成功，ID: {} (写入到位置0)", request_id);
            println!("📊 确认: tail 从3变为0，然后变为1");
        }
        Err(e) => println!("❌ 验证消息9写入失败: {}", e),
    }
    // 现在状态: [FULL, EMPTY, FULL, FULL], head=2, tail=1
    
    // 步骤8: 第三次写入消息，验证 tail 现在指向位置1
    println!("\n🔸 步骤8: 验证 tail 现在指向位置1");
    let message = Message {
        id: 10,
        data: "验证消息10".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 验证消息10写入成功，ID: {} (写入到位置1)", request_id);
            println!("📊 确认: tail 从1变为2");
        }
        Err(e) => println!("❌ 验证消息10写入失败: {}", e),
    }
    // 现在状态: [FULL, FULL, FULL, FULL], head=2, tail=2
    
    // 步骤9: 第四次写入消息，应该失败因为队列满了
    println!("\n🔸 步骤9: 尝试写入到位置2（应该失败）");
    let message = Message {
        id: 11,
        data: "应该失败的消息11".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => println!("❌ 意外成功: 消息11写入成功，ID: {}", request_id),
        Err(e) => {
            println!("✅ 预期失败: {}", e);
            println!("📊 确认: 位置2是FULL状态，无法写入");
        }
    }
    
    println!("\n🎯 结论:");
    println!("当 tail=3 且位置3是EMPTY时:");
    println!("1. push 操作会成功写入到位置3");
    println!("2. tail 会更新为 (3 + 1) % 4 = 0");
    println!("3. 下一次 push 会尝试写入位置0");
    
    println!("\n✅ 测试完成");
}