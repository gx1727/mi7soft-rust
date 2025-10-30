use crate::shared_slot::SlotState;
use crate::{Message, SharedSlotPipe};

use std::ptr::NonNull;
use std::str::FromStr;
use std::sync::atomic::AtomicU32;

/// 动态管道trait，定义所有管道类型的通用接口
pub trait DynamicPipe: Send + Sync {
    /// 获取空槽位
    fn hold(&self) -> Result<usize, Box<dyn std::error::Error>>;

    /// 发送消息
    fn send(&self, index: usize, message: Message) -> Result<u64, Box<dyn std::error::Error>>;

    /// 获取消息
    fn fetch(&self) -> Result<usize, Box<dyn std::error::Error>>;

    /// 接收消息
    fn receive(&self, index: usize) -> Result<Message, Box<dyn std::error::Error>>;

    /// 设置槽位状态
    fn set_slot_state(
        &self,
        index: usize,
        state: SlotState,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// 获取槽位状态
    fn get_slot_state(&self, index: usize) -> Result<SlotState, Box<dyn std::error::Error>>;

    /// 获取管道状态
    fn status(&self) -> PipeStatus;

    /// 获取配置信息
    fn config(&self) -> PipeConfig;

    /// 获取容量
    fn capacity(&self) -> usize;

    /// 获取槽位大小
    fn slot_size(&self) -> usize;
}

/// 管道类型枚举，支持预定义和自定义配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeType {
    /// 小型管道：10槽位 x 1KB
    Small,
    /// 默认管道：100槽位 x 4KB  
    Default,
    /// 大型管道：1000槽位 x 8KB
    Large,
    /// 自定义管道：指定容量和槽位大小
    Custom(usize, usize),
}

impl PipeType {
    /// 获取管道类型对应的配置
    pub fn config(&self) -> PipeConfig {
        match self {
            PipeType::Small => PipeConfig::small(),
            PipeType::Default => PipeConfig::default(),
            PipeType::Large => PipeConfig::large(),
            PipeType::Custom(capacity, slot_size) => PipeConfig::new(*capacity, *slot_size),
        }
    }

    /// 从配置创建管道类型
    pub fn from_config(config: PipeConfig) -> Self {
        match (config.capacity, config.slot_size) {
            (10, 1024) => PipeType::Small,
            (100, 4096) => PipeType::Default,
            (1000, 8192) => PipeType::Large,
            (capacity, slot_size) => PipeType::Custom(capacity, slot_size),
        }
    }

    /// 获取所有支持的字符串类型
    pub fn supported_types() -> Vec<&'static str> {
        vec!["small", "default", "large"]
    }
}

impl FromStr for PipeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "small" => Ok(PipeType::Small),
            "default" => Ok(PipeType::Default),
            "large" => Ok(PipeType::Large),
            _ => Err(format!(
                "不支持的管道类型: '{}'. 支持的类型: {:?}",
                s,
                PipeType::supported_types()
            )),
        }
    }
}

impl std::fmt::Display for PipeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeType::Small => write!(f, "small"),
            PipeType::Default => write!(f, "default"),
            PipeType::Large => write!(f, "large"),
            PipeType::Custom(capacity, slot_size) => {
                write!(f, "custom({}x{})", capacity, slot_size)
            }
        }
    }
}

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

    /// 从字符串创建配置
    pub fn from_str(s: &str) -> Result<Self, String> {
        let pipe_type = PipeType::from_str(s)?;
        Ok(pipe_type.config())
    }

    /// 根据字符串类型创建管道
    pub fn create_pipe_from_str(
        &self,
        pipe_type_str: &str,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        PipeFactory::create_pipe_from_str(pipe_type_str, name)
    }

    /// 根据字符串类型连接管道
    pub fn connect_pipe_from_str(
        &self,
        pipe_type_str: &str,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        PipeFactory::connect_pipe_from_str(pipe_type_str, name)
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), String> {
        if self.capacity == 0 {
            return Err("容量不能为0".to_string());
        }
        if self.slot_size == 0 {
            return Err("槽大小不能为0".to_string());
        }
        if self.capacity > 10000 {
            return Err("容量过大，最大支持10000".to_string());
        }
        if self.slot_size > 1024 * 1024 {
            return Err("槽大小过大，最大支持1MB".to_string());
        }
        Ok(())
    }

    /// 计算总内存使用量（字节）
    pub fn total_memory(&self) -> usize {
        self.capacity * self.slot_size
    }

    /// 检查是否为预定义配置
    pub fn is_predefined(&self) -> bool {
        matches!(
            (self.capacity, self.slot_size),
            (10, 1024) | (100, 4096) | (1000, 8192)
        )
    }

    /// 获取配置类型描述
    pub fn config_type(&self) -> &'static str {
        match (self.capacity, self.slot_size) {
            (10, 1024) => "Small",
            (100, 4096) => "Default",
            (1000, 8192) => "Large",
            _ => "Custom",
        }
    }

    /// 转换为PipeType
    pub fn to_pipe_type(&self) -> PipeType {
        match (self.capacity, self.slot_size) {
            (10, 1024) => PipeType::Small,
            (100, 4096) => PipeType::Default,
            (1000, 8192) => PipeType::Large,
            (capacity, slot_size) => PipeType::Custom(capacity, slot_size),
        }
    }

    /// 创建管道（使用工厂）
    pub fn create_pipe(
        &self,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        self.validate().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                as Box<dyn std::error::Error>
        })?;
        PipeFactory::create_with_config(*self, name)
    }

    /// 连接到管道（使用工厂）
    pub fn connect_pipe(
        &self,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        self.validate().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                as Box<dyn std::error::Error>
        })?;
        PipeFactory::connect_with_config(*self, name)
    }

    /// 比较两个配置是否兼容
    pub fn is_compatible(&self, other: &PipeConfig) -> bool {
        self.capacity == other.capacity && self.slot_size == other.slot_size
    }

    /// 创建性能优化的配置建议
    pub fn performance_suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        if self.total_memory() > 100 * 1024 * 1024 {
            // 100MB
            suggestions.push("内存使用量较大，考虑减少容量或槽大小".to_string());
        }

        if self.capacity > 1000 && self.slot_size < 1024 {
            suggestions.push("高容量配置建议使用更大的槽大小以提高性能".to_string());
        }

        if self.capacity < 10 && self.slot_size > 8192 {
            suggestions.push("低容量配置使用大槽大小可能造成内存浪费".to_string());
        }

        if !self.is_predefined() {
            suggestions.push("考虑使用预定义配置以获得更好的性能优化".to_string());
        }

        suggestions
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
                .map_err(|e| format!("Failed to create shared pipe: {:?}", e))?;

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
                .map_err(|e| format!("Failed to connect to shared pipe: {:?}", e))?;

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

    /// 接收消息
    pub fn receive(&self, index: usize) -> Result<Message, Box<dyn std::error::Error>> {
        unsafe {
            let pipe = self.pipe.as_ptr();
            match (*pipe).read::<Message>(index) {
                Ok(Some((_, message))) => Ok(message),
                Ok(None) => Err("槽位为空，无法读取消息".into()),
                Err(err) => Err(err.into()),
            }
        }
    }

    /// 尝试接收消息（非阻塞，返回Option）
    pub fn try_receive(&self, index: usize) -> Result<Option<Message>, Box<dyn std::error::Error>> {
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

/// 为CrossProcessPipe实现DynamicPipe trait
impl<const CAPACITY: usize, const SLOT_SIZE: usize> DynamicPipe
    for CrossProcessPipe<CAPACITY, SLOT_SIZE>
{
    fn hold(&self) -> Result<usize, Box<dyn std::error::Error>> {
        self.hold()
    }

    fn send(&self, index: usize, message: Message) -> Result<u64, Box<dyn std::error::Error>> {
        self.send(index, message)
    }

    fn fetch(&self) -> Result<usize, Box<dyn std::error::Error>> {
        self.fetch()
    }

    fn receive(&self, index: usize) -> Result<Message, Box<dyn std::error::Error>> {
        self.receive(index)
    }

    fn set_slot_state(
        &self,
        index: usize,
        state: SlotState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_slot_state(index, state)
    }

    fn get_slot_state(&self, index: usize) -> Result<SlotState, Box<dyn std::error::Error>> {
        self.get_slot_state(index)
    }

    fn status(&self) -> PipeStatus {
        self.status()
    }

    fn config(&self) -> PipeConfig {
        self.config()
    }

    fn capacity(&self) -> usize {
        self.capacity()
    }

    fn slot_size(&self) -> usize {
        self.slot_size()
    }
}

/// 动态管道工厂，支持根据配置创建不同类型的管道
pub struct PipeFactory;

impl PipeFactory {
    /// 根据字符串类型创建管道
    pub fn create_pipe_from_str(
        pipe_type_str: &str,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        let pipe_type = PipeType::from_str(pipe_type_str)?;
        Self::create_pipe(pipe_type, name)
    }

    /// 根据字符串类型连接到现有管道
    pub fn connect_pipe_from_str(
        pipe_type_str: &str,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        let pipe_type = PipeType::from_str(pipe_type_str)?;
        Self::connect_pipe(pipe_type, name)
    }

    /// 根据管道类型创建管道
    pub fn create_pipe(
        pipe_type: PipeType,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        match pipe_type {
            PipeType::Small => {
                let pipe = CrossProcessPipe::<10, 1024>::create(name)?;
                Ok(Box::new(pipe))
            }
            PipeType::Default => {
                let pipe = CrossProcessPipe::<100, 4096>::create(name)?;
                Ok(Box::new(pipe))
            }
            PipeType::Large => {
                let pipe = CrossProcessPipe::<1000, 8192>::create(name)?;
                Ok(Box::new(pipe))
            }
            PipeType::Custom(capacity, slot_size) => {
                // 对于自定义配置，我们需要使用宏或者匹配常见的配置
                Self::create_custom_pipe(capacity, slot_size, name)
            }
        }
    }

    /// 根据配置创建管道
    pub fn create_with_config(
        config: PipeConfig,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        let pipe_type = PipeType::from_config(config);
        Self::create_pipe(pipe_type, name)
    }

    /// 连接到现有管道
    pub fn connect_pipe(
        pipe_type: PipeType,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        match pipe_type {
            PipeType::Small => {
                let pipe = CrossProcessPipe::<10, 1024>::connect(name)?;
                Ok(Box::new(pipe))
            }
            PipeType::Default => {
                let pipe = CrossProcessPipe::<100, 4096>::connect(name)?;
                Ok(Box::new(pipe))
            }
            PipeType::Large => {
                let pipe = CrossProcessPipe::<1000, 8192>::connect(name)?;
                Ok(Box::new(pipe))
            }
            PipeType::Custom(capacity, slot_size) => {
                Self::connect_custom_pipe(capacity, slot_size, name)
            }
        }
    }

    /// 根据配置连接到现有管道
    pub fn connect_with_config(
        config: PipeConfig,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        let pipe_type = PipeType::from_config(config);
        Self::connect_pipe(pipe_type, name)
    }

    /// 创建自定义配置的管道（支持常见配置）
    fn create_custom_pipe(
        capacity: usize,
        slot_size: usize,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        match (capacity, slot_size) {
            // 常见的自定义配置
            (50, 2048) => {
                let pipe = CrossProcessPipe::<50, 2048>::create(name)?;
                Ok(Box::new(pipe))
            }
            (200, 1024) => {
                let pipe = CrossProcessPipe::<200, 1024>::create(name)?;
                Ok(Box::new(pipe))
            }
            (500, 512) => {
                let pipe = CrossProcessPipe::<500, 512>::create(name)?;
                Ok(Box::new(pipe))
            }
            (20, 16384) => {
                let pipe = CrossProcessPipe::<20, 16384>::create(name)?;
                Ok(Box::new(pipe))
            }
            // 如果是预定义配置，重定向到对应类型
            (10, 1024) => {
                let pipe = CrossProcessPipe::<10, 1024>::create(name)?;
                Ok(Box::new(pipe))
            }
            (100, 4096) => {
                let pipe = CrossProcessPipe::<100, 4096>::create(name)?;
                Ok(Box::new(pipe))
            }
            (1000, 8192) => {
                let pipe = CrossProcessPipe::<1000, 8192>::create(name)?;
                Ok(Box::new(pipe))
            }
            _ => {
                Err(format!("不支持的自定义配置: capacity={}, slot_size={}. 请使用预定义配置或添加支持的自定义配置", capacity, slot_size).into())
            }
        }
    }

    /// 连接到自定义配置的管道
    fn connect_custom_pipe(
        capacity: usize,
        slot_size: usize,
        name: &str,
    ) -> Result<Box<dyn DynamicPipe>, Box<dyn std::error::Error>> {
        match (capacity, slot_size) {
            // 常见的自定义配置
            (50, 2048) => {
                let pipe = CrossProcessPipe::<50, 2048>::connect(name)?;
                Ok(Box::new(pipe))
            }
            (200, 1024) => {
                let pipe = CrossProcessPipe::<200, 1024>::connect(name)?;
                Ok(Box::new(pipe))
            }
            (500, 512) => {
                let pipe = CrossProcessPipe::<500, 512>::connect(name)?;
                Ok(Box::new(pipe))
            }
            (20, 16384) => {
                let pipe = CrossProcessPipe::<20, 16384>::connect(name)?;
                Ok(Box::new(pipe))
            }
            // 如果是预定义配置，重定向到对应类型
            (10, 1024) => {
                let pipe = CrossProcessPipe::<10, 1024>::connect(name)?;
                Ok(Box::new(pipe))
            }
            (100, 4096) => {
                let pipe =  CrossProcessPipe::<100, 4096>::connect(name)?;
                Ok(Box::new(pipe))
            }
            (1000, 8192) => {
                let pipe = CrossProcessPipe::<1000, 8192>::connect(name)?;
                Ok(Box::new(pipe))
            }
            _ => {
                Err(format!("不支持的自定义配置: capacity={}, slot_size={}. 请使用预定义配置或添加支持的自定义配置", capacity, slot_size).into())
            }
        }
    }
}
