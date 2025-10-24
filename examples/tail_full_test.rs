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
        let ptr = unsafe { SharedRingQueue::open("/test_tail_full", true) };
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
    println!("🧪 测试当 tail 指向 FULL 状态位置时的行为");
    
    let queue = Arc::new(Mutex::new(unsafe { SafeSharedRing::new() }));
    
    println!("\n📝 构造场景: [FULL, EMPTY, EMPTY, FULL] -> tail=3 -> push -> tail=0(FULL)");
    
    // 步骤1: 填满队列
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
    
    // 步骤2: 消费位置0、1、2的消息
    println!("\n🔸 步骤2: 消费位置0、1、2的消息");
    for i in 1..=3 {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息 {}: ID={}, 内容={}", i, request_id, content);
        }
    }
    // 状态: [EMPTY, EMPTY, EMPTY, FULL], head=3, tail=0
    
    // 步骤3: 写入消息到位置0、1、2
    println!("\n🔸 步骤3: 写入消息到位置0、1、2");
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
    // 状态: [FULL, FULL, FULL, FULL], head=3, tail=3
    
    // 步骤4: 消费位置3的消息
    println!("\n🔸 步骤4: 消费位置3的消息");
    {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费位置3消息: ID={}, 内容={}", request_id, content);
        }
    }
    // 状态: [FULL, FULL, FULL, EMPTY], head=0, tail=3
    
    // 步骤5: 消费位置0、1的消息
    println!("\n🔸 步骤5: 消费位置0、1的消息");
    for i in 0..=1 {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费位置{}消息: ID={}, 内容={}", i, request_id, content);
        }
    }
    // 状态: [EMPTY, EMPTY, FULL, EMPTY], head=2, tail=3
    
    println!("\n📊 当前状态: [EMPTY, EMPTY, FULL, EMPTY], head=2, tail=3");
    
    // 步骤6: 写入消息到位置3，tail会变为0
    println!("\n🔸 步骤6: 写入消息到位置3 (tail会从3变为0)");
    let message = Message {
        id: 8,
        data: "消息8".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 消息8写入成功，ID: {} (写入到位置3)", request_id);
            println!("📊 tail 现在指向位置0，但位置0是EMPTY");
        }
        Err(e) => println!("❌ 消息8写入失败: {}", e),
    }
    // 状态: [EMPTY, EMPTY, FULL, FULL], head=2, tail=0
    
    // 步骤7: 写入消息到位置0，tail会变为1
    println!("\n🔸 步骤7: 写入消息到位置0 (tail会从0变为1)");
    let message = Message {
        id: 9,
        data: "消息9".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 消息9写入成功，ID: {} (写入到位置0)", request_id);
            println!("📊 tail 现在指向位置1，但位置1是EMPTY");
        }
        Err(e) => println!("❌ 消息9写入失败: {}", e),
    }
    // 状态: [FULL, EMPTY, FULL, FULL], head=2, tail=1
    
    // 步骤8: 写入消息到位置1，tail会变为2
    println!("\n🔸 步骤8: 写入消息到位置1 (tail会从1变为2)");
    let message = Message {
        id: 10,
        data: "消息10".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 消息10写入成功，ID: {} (写入到位置1)", request_id);
            println!("📊 tail 现在指向位置2，位置2是FULL！");
        }
        Err(e) => println!("❌ 消息10写入失败: {}", e),
    }
    // 状态: [FULL, FULL, FULL, FULL], head=2, tail=2
    
    // 🎯 关键测试：tail指向FULL状态的位置2
    println!("\n🎯 关键测试: tail=2，位置2是FULL状态");
    println!("🔸 步骤9: 尝试写入到位置2 (应该失败!)");
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
        Ok(request_id) => {
            println!("❌ 意外成功: 消息11写入成功，ID: {}", request_id);
            println!("🚨 这不应该发生！");
        }
        Err(e) => {
            println!("✅ 预期失败: {}", e);
            println!("📊 确认: 当 tail 指向 FULL 状态位置时，push 会立即失败");
            println!("📊 错误信息: '{}'", e);
            println!("📊 tail 保持不变，仍然指向位置2");
        }
    }
    
    println!("\n🎯 结论:");
    println!("当 tail 指向 FULL 状态的位置时:");
    println!("1. push 方法会检查 self.slots[self.tail].state");
    println!("2. 如果不是 EMPTY，立即返回 '队列已满' 错误");
    println!("3. tail 指针不会移动");
    println!("4. 即使队列中有其他 EMPTY 位置，也无法写入");
    
    // 验证：消费一个消息后再尝试写入
    println!("\n🔸 验证: 消费位置2的消息后再尝试写入");
    {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费位置2消息: ID={}, 内容={}", request_id, content);
        }
    }
    // 状态: [FULL, FULL, EMPTY, FULL], head=3, tail=2
    
    println!("\n🔸 现在再次尝试写入到位置2");
    let message = Message {
        id: 12,
        data: "现在应该成功的消息12".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 消息12写入成功，ID: {} (写入到位置2)", request_id);
            println!("📊 确认: 位置2变为EMPTY后，可以成功写入");
        }
        Err(e) => println!("❌ 消息12写入失败: {}", e),
    }
    
    println!("\n✅ 测试完成");
}