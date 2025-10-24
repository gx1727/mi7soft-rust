use mi7::{Message, SharedRingQueue};
use std::sync::{Arc, Mutex};

// 安全包装器
struct SafeSharedRing {
    ptr: *mut SharedRingQueue<4, 1024>, // 小队列，容易测试满的情况
}

unsafe impl Send for SafeSharedRing {}
unsafe impl Sync for SafeSharedRing {}

impl SafeSharedRing {
    unsafe fn new() -> Self {
        let ptr = unsafe { SharedRingQueue::open("/test_queue_full", true) };
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
    println!("🧪 测试队列满的逻辑");

    let queue = Arc::new(Mutex::new(unsafe { SafeSharedRing::new() }));

    // 测试：填满队列
    println!("\n📝 步骤1: 尝试填满队列（容量=4）");
    for i in 1..=5 {
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
                break;
            }
        }
    }

    // 测试：消费一个消息后再写入
    println!("\n📖 步骤2: 消费一个消息");
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

    // 测试：再次尝试写入
    println!("\n📝 步骤3: 消费后再次尝试写入");
    let message = Message {
        id: 6,
        data: "消息 6".as_bytes().to_vec(),
        timestamp: 0,
    };

    let result = {
        let mut queue_guard = queue.lock().unwrap();
        unsafe { queue_guard.push(&message) }
    };

    match result {
        Ok(request_id) => {
            println!("✅ 消息 6 写入成功，请求 ID: {}", request_id);
        }
        Err(e) => {
            println!("❌ 消息 6 写入失败: {}", e);
        }
    }

    // 测试：显示最终状态
    println!("\n📊 步骤4: 消费剩余消息");
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
