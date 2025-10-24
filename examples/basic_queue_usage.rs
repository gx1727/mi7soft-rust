use mi7::shared_ring::SharedRingQueue;
use std::sync::Arc;

#[derive(Debug, Clone, bincode::Encode, bincode::Decode)]
struct Message {
    id: u64,
    content: String,
}

// 包装器结构体，用于安全地在线程间传递共享内存指针
struct SafeSharedRing {
    ptr: *mut SharedRingQueue<1024, 256>,
}

unsafe impl Send for SafeSharedRing {}
unsafe impl Sync for SafeSharedRing {}

impl SafeSharedRing {
    fn new(name: &str) -> Self {
        let ptr = unsafe { SharedRingQueue::<1024, 256>::open(name, true) };
        Self { ptr }
    }

    unsafe fn push<T: bincode::Encode>(&mut self, value: &T) -> Result<u64, &'static str> {
        unsafe { (*self.ptr).push(value) }
    }

    unsafe fn pop<T: bincode::Decode<()>>(&mut self) -> Option<(u64, T)> {
        unsafe { (*self.ptr).pop::<T>() }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动简单测试");

    // 创建共享内存环形队列
    let queue = Arc::new(std::sync::Mutex::new(SafeSharedRing::new("/simple_test")));

    // 测试写入
    let message = Message {
        id: 1,
        content: "测试消息".to_string(),
    };

    {
        let mut queue_guard = queue.lock().unwrap();
        match unsafe { queue_guard.push(&message) } {
            Ok(request_id) => println!("✅ 写入成功，请求 ID: {}", request_id),
            Err(e) => println!("❌ 写入失败: {}", e),
        }
    }

    // 测试读取
    {
        let mut queue_guard = queue.lock().unwrap();
        match unsafe { queue_guard.pop::<Message>() } {
            Some((request_id, msg)) => {
                println!("✅ 读取成功，请求 ID: {}, 消息: {:?}", request_id, msg);
            }
            None => println!("❌ 队列为空"),
        }
    }

    println!("✅ 测试完成");
    Ok(())
}
