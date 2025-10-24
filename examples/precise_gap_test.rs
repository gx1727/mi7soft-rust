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
        let ptr = unsafe { SharedRingQueue::open("/test_precise_gap", true) };
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
    println!("🧪 精确测试你描述的场景");
    
    let queue = Arc::new(Mutex::new(unsafe { SafeSharedRing::new() }));
    
    // 步骤1: 填满队列 [1,2,3,4]
    println!("\n📝 步骤1: 填满队列");
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
        
        println!("✅ 消息 {} 写入: {:?}", i, result);
    }
    
    println!("📊 队列已填满");
    
    // 步骤2: 只消费第一个消息，让位置1变为EMPTY
    println!("\n📖 步骤2: 只消费第一个消息");
    {
        let mut queue_guard = queue.lock().unwrap();
        if let Some((request_id, message)) = unsafe { queue_guard.pop::<Message>() } {
            let content = String::from_utf8_lossy(&message.data);
            println!("✅ 消费消息: ID={}, 内容={}", request_id, content);
        }
        println!("📊 已消费第一个消息");
    }
    
    // 现在状态应该是：
    // 队列：[EMPTY, FULL, FULL, FULL] (位置0,1,2,3)
    // head=1, tail=0
    
    // 步骤3: 尝试写入新消息
    println!("\n📝 步骤3: 尝试写入新消息");
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
        Ok(request_id) => {
            println!("✅ 新消息写入成功，请求 ID: {}", request_id);
        }
        Err(e) => {
            println!("❌ 新消息写入失败: {}", e);
        }
    }
    
    // 步骤4: 再次尝试写入，这次应该检查位置1
    println!("\n📝 步骤4: 再次尝试写入（检查位置1）");
    let message = Message {
        id: 6,
        data: "新消息6".as_bytes().to_vec(),
        timestamp: 0,
    };
    
    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };
    
    match result {
        Ok(request_id) => {
            println!("✅ 新消息写入成功，请求 ID: {}", request_id);
        }
        Err(e) => {
            println!("❌ 新消息写入失败: {} (这里应该失败，因为位置1是FULL)", e);
        }
    }
    
    // 步骤5: 消费剩余消息验证
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