use std::fmt;

// 消息类型（请求/响应）
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub enum MessageType {
    Request,
    Response,
}

// 共享内存中的消息结构
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Message {
    pub id: u64,              // 消息ID（用于关联请求和响应）
    pub msg_type: MessageType, // 消息类型
    pub data: [u8; 256],      // 消息内容（固定大小的字节数组）
    pub data_len: usize,      // 实际数据长度
}

impl Message {
    pub fn new(id: u64, msg_type: MessageType, data: &str) -> Self {
        let mut msg = Message {
            id,
            msg_type,
            data: [0; 256],
            data_len: 0,
        };
        msg.set_data(data);
        msg
    }
    
    pub fn set_data(&mut self, data: &str) {
        let bytes = data.as_bytes();
        let len = bytes.len().min(255); // 保留一个字节用于null终止符
        self.data[..len].copy_from_slice(&bytes[..len]);
        self.data[len] = 0; // null终止符
        self.data_len = len;
    }
    
    pub fn get_data(&self) -> String {
        String::from_utf8_lossy(&self.data[..self.data_len]).to_string()
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Message(id={}, type={:?}, data={})",
            self.id, self.msg_type, self.get_data()
        )
    }
}

// 共享内存中循环缓冲区的配置
pub const BUFFER_SIZE: usize = 10; // 最多存储10条消息
pub const SHARED_MEM_NAME: &str = "/my_framework_shm"; // 共享内存名称（Linux）
pub const WINDOWS_SHARED_MEM_NAME: &str = "Global\\MyFrameworkShm"; // Windows 共享内存名称
pub const MUTEX_NAME: &str = "/my_framework_mutex"; // 互斥锁名称（Linux）
pub const WINDOWS_MUTEX_NAME: &str = "Global\\MyFrameworkMutex"; // Windows 互斥锁名称