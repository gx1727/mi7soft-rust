use crate::shared_slot::SlotState;
use crate::{Message, SharedSlotPipe};
use std::ptr::NonNull;

#[derive(Debug, Clone)]
pub struct PipeStatus {
    /// 队列槽位总数量
    pub capacity: usize,
    /// 每个槽位的数据大小（字节）
    pub slot_size: usize,
    /// 写指针位置
    pub write_pointer: usize,
    /// 读指针位置
    pub read_pointer: usize,
    /// EMPTY 状态的槽位数量
    pub empty_count: usize,
    /// WRITING 状态的槽位数量
    pub writing_count: usize,
    /// INPROGRESS 状态的槽位数量
    pub in_progress_count: usize,
    /// READING 状态的槽位数量
    pub reading_count: usize,
    /// READY 状态的槽位数量
    pub ready_count: usize,
    /// 已使用的槽位数量（非 EMPTY 状态的槽位）
    pub used_count: usize,
}

/// 队列配置结构体
#[derive(Debug, Clone, Copy)]
pub struct PipeConfig {
    /// 队列槽位数量
    pub capacity: usize,
    /// 每个槽位的大小（字节）
    pub slot_size: usize,
}

impl PipeConfig {
    /// 创建新的队列配置
    pub fn new(capacity: usize, slot_size: usize) -> Self {
        Self {
            capacity,
            slot_size,
        }
    }

    /// 默认配置：100个槽位，每个4KB
    pub fn default() -> Self {
        Self {
            capacity: 100,
            slot_size: 4096,
        }
    }

    /// 小型队列配置：10个槽位，每个1KB
    pub fn small() -> Self {
        Self {
            capacity: 10,
            slot_size: 1024,
        }
    }

    /// 大型队列配置：1000个槽位，每个8KB
    pub fn large() -> Self {
        Self {
            capacity: 1000,
            slot_size: 8192,
        }
    }
}

/// 跨进程Slot包装器，提供类似CrossProcessSlot的API
/// 支持配置化的队列大小和槽位大小
pub struct CrossProcessPipe<const CAPACITY: usize, const SLOT_SIZE: usize> {
    pipe: NonNull<SharedSlotPipe<CAPACITY, SLOT_SIZE>>,
    _name: String,
    config: PipeConfig,
}

unsafe impl<const CAPACITY: usize, const SLOT_SIZE: usize> Send
    for CrossProcessPipe<CAPACITY, SLOT_SIZE>
{
}
unsafe impl<const CAPACITY: usize, const SLOT_SIZE: usize> Sync
    for CrossProcessPipe<CAPACITY, SLOT_SIZE>
{
}

impl<const CAPACITY: usize, const SLOT_SIZE: usize> CrossProcessPipe<CAPACITY, SLOT_SIZE> {
    /// 创建新的队列
    pub fn create(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let pipe_ptr = SharedSlotPipe::<CAPACITY, SLOT_SIZE>::open(name, true)
                .map_err(|e| format!("Failed to create shared ring queue: {:?}", e))?;

            Ok(Self {
                pipe: NonNull::new_unchecked(pipe_ptr),
                _name: name.to_string(),
                config: PipeConfig::new(CAPACITY, SLOT_SIZE),
            })
        }
    }

    /// 使用配置创建新的队列
    pub fn create_with_config(
        name: &str,
        _config: PipeConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 注意：配置参数在编译时确定，这里主要用于验证
        if _config.capacity != CAPACITY || _config.slot_size != SLOT_SIZE {
            return Err(format!(
                "配置不匹配：期望 capacity={}, slot_size={}，实际 capacity={}, slot_size={}",
                CAPACITY, SLOT_SIZE, _config.capacity, _config.slot_size
            )
            .into());
        }
        Self::create(name)
    }

    /// 连接到现有队列
    pub fn connect(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let pipe_ptr = SharedSlotPipe::<CAPACITY, SLOT_SIZE>::open(name, false)
                .map_err(|e| format!("Failed to connect to shared ring queue: {:?}", e))?;

            Ok(Self {
                pipe: NonNull::new_unchecked(pipe_ptr),
                _name: name.to_string(),
                config: PipeConfig::new(CAPACITY, SLOT_SIZE),
            })
        }
    }

    /// 获取 空slot
    pub fn hold(&self) -> Result<usize, Box<dyn std::error::Error>> {
        unsafe {
            let pipe = self.pipe.as_ptr();
            match (*pipe).hold() {
                Some(index) => Ok(index),
                None => Err("队列已满，无法获取空槽位".into()),
            }
        }
    }
    /// 发送消息
    /// 将数据写入slot
    pub fn send(&self, index: usize, message: Message) -> Result<u64, Box<dyn std::error::Error>> {
        unsafe {
            let pipe = self.pipe.as_ptr();
            match (*pipe).write(index, &message) {
                Ok(request_id) => Ok(request_id),
                Err(err) => Err(err.into()),
            }
        }
    }

    /// 接收消息
    pub fn fetch(&self) -> Result<usize, Box<dyn std::error::Error>> {
        unsafe {
            let pipe = self.pipe.as_ptr();
            match (*pipe).fetch() {
                Some(index) => Ok(index),
                None => Err("队列为空，无法获取消息".into()),
            }
        }
    }

    /// 尝试接收消息（非阻塞）
    pub fn receive(&self, index: usize) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        unsafe {
            let pipe = self.pipe.as_ptr();
            match (*pipe).read::<Message>(index) {
                Ok(Some((_, message))) => Ok(Some(message)),
                Ok(None) => Ok(None),
                Err(err) => Err(err.into()),
            }
        }
    }

    /// 获取队列状态
    pub fn status(&self) -> PipeStatus {
        unsafe {
            let pipe = self.pipe.as_ptr();

            // 获取写指针和读指针
            let write_pointer = (*pipe).write_pointer;
            let read_pointer = (*pipe).read_pointer;

            // 统计各种状态的槽位数量
            let mut empty_count = 0;
            let mut writing_count = 0;
            let mut in_progress_count = 0;
            let mut reading_count = 0;
            let mut ready_count = 0;

            // 遍历所有槽位统计状态
            for i in 0..CAPACITY {
                match (*pipe).slots[i]
                    .state
                    .load(std::sync::atomic::Ordering::Acquire)
                {
                    x if x == SlotState::EMPTY as u32 => empty_count += 1,
                    x if x == SlotState::WRITING as u32 => writing_count += 1,
                    x if x == SlotState::INPROGRESS as u32 => in_progress_count += 1,
                    x if x == SlotState::READING as u32 => reading_count += 1,
                    x if x == SlotState::READY as u32 => ready_count += 1,
                    _ => {} // 未知状态，忽略
                }
            }

            let used_count = CAPACITY - empty_count;

            PipeStatus {
                capacity: CAPACITY,
                slot_size: SLOT_SIZE,
                write_pointer,
                read_pointer,
                empty_count,
                writing_count,
                in_progress_count,
                reading_count,
                ready_count,
                used_count,
            }
        }
    }

    /// 获取队列配置
    pub fn config(&self) -> PipeConfig {
        self.config
    }

    /// 获取队列容量
    pub fn capacity(&self) -> usize {
        CAPACITY
    }

    /// 获取槽位大小
    pub fn slot_size(&self) -> usize {
        SLOT_SIZE
    }

    /// 设置槽位状态（用于调度者）
    pub fn set_slot_state(
        &self,
        index: usize,
        state: SlotState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let queue = self.pipe.as_ptr();
            (*queue).set_slot_state(index, state).map_err(|e| e.into())
        }
    }

    /// 获取槽位状态
    pub fn get_slot_state(&self, index: usize) -> Result<SlotState, Box<dyn std::error::Error>> {
        unsafe {
            let queue = self.pipe.as_ptr();
            (*queue).get_slot_state(index).map_err(|e| e.into())
        }
    }
}

// 为了向后兼容，提供默认配置的类型别名
pub type DefaultCrossProcessPipe = CrossProcessPipe<100, 4096>;

// 提供一些常用配置的类型别名
pub type SmallCrossProcessPipe = CrossProcessPipe<10, 1024>;
pub type LargeCrossProcessPipe = CrossProcessPipe<1000, 8192>;

// 为默认配置提供便捷方法
impl DefaultCrossProcessPipe {
    /// 使用默认配置创建队列
    pub fn create_default(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::create(name)
    }

    /// 使用默认配置连接队列
    pub fn connect_default(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect(name)
    }
}
