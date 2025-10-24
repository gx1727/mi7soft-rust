use mi7::{SharedRingQueue, Message};
use std::sync::{Arc, Mutex};

// 安全包装器
struct SafeSharedRing {
    ptr: *mut SharedRingQueue<4, 1024>, // 小队列，容易测试
}

unsafe impl Send for SafeSharedRing {}
unsafe impl Sync for SafeSharedRing {}

impl SafeSharedRing {
    unsafe fn new() -> Self {
        let ptr = unsafe { SharedRingQueue::open("/test_queue_gap", true) };
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
    println!("🧪 测试队列空隙问题");
    
    let queue = Arc::new(Mutex::new(unsafe { SafeSharedRing::new() }));
    
    // 步骤1: 填满队列
    println!("\n📝 步骤1: 填满队列（容量=4）");
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
            Ok(request_id) => {
                println!("✅ 消息 {} 写入成功，请求 ID: {}", i, request_id);
            }
            Err(e) => {
                println!("❌ 消息 {} 写入失败: {}", i, e);
            }
        }
    }
    
    // 步骤2: 消费第一个消息（位置1变为EMPTY）
    println!("\n📖 步骤2: 消费第一个消息");
    {
        let mut queue_guard = queue.lock().unwrap();
        match unsafe { queue_guard.pop::<Message>() } {
            Some((request_id, message)) => {
                let content = String::from_utf8_lossy(&message.data);
                println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
            }
            None => {
                println!("❌ 队列为空，无法消费");
            }
        }
    }
    
    // 步骤3: 消费第三个消息（位置3变为EMPTY，但位置2还是FULL）
    println!("\n📖 步骤3: 消费第三个消息（跳过第二个）");
    {
        let mut queue_guard = queue.lock().unwrap();
        // 先消费第二个
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
        }
        // 再消费第三个
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
        }
    }
    
    // 现在状态应该是：[EMPTY, EMPTY, EMPTY, FULL]，tail指向位置1
    
    // 步骤4: 尝试写入新消息
    println!("\n📝 步骤4: 尝试写入新消息（应该成功，因为有空位）");
    for i in 5..=7 {
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
            Ok(request_id) => {
                println!("✅ 消息 {} 写入成功，请求 ID: {}", i, request_id);
            }
            Err(e) => {
                println!("❌ 消息 {} 写入失败: {} (这里暴露了问题！)", i, e);
                break;
            }
        }
    }
    
    // 步骤5: 消费剩余消息
    println!("\n📊 步骤5: 消费剩余消息");
    loop {
        let result = {
            let mut queue_guard = queue.lock().unwrap();
            unsafe { queue_guard.pop::<Message>() }
        };
        
        match result {
            Some((request_id, message)) => {
                let content = String::from_utf8_lossy(&message.data);
                println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
            }
            None => {
                println!("📭 队列已空");
                break;
            }
        }
    }
    
    println!("\n✅ 测试完成");
}