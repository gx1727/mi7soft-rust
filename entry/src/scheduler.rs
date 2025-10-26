use mi7::DefaultCrossProcessPipe;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// 调度者结构体，管理槽位分配
pub struct Scheduler {
    queue: Arc<DefaultCrossProcessPipe>,
    counter: Arc<AtomicI64>,
    slot_sender: mpsc::UnboundedSender<usize>,
    slot_receiver: mpsc::UnboundedReceiver<usize>,
}

impl Scheduler {
    /// 创建新的调度者实例
    pub fn new(queue: Arc<DefaultCrossProcessPipe>) -> Self {
        let (slot_sender, slot_receiver) = mpsc::unbounded_channel();
        let counter = Arc::new(AtomicI64::new(0));

        Self {
            queue,
            counter,
            slot_sender,
            slot_receiver,
        }
    }

    /// 获取计数器的克隆
    pub fn get_counter(&self) -> Arc<AtomicI64> {
        self.counter.clone()
    }

    /// 获取槽位发送器的克隆
    pub fn get_slot_sender(&self) -> mpsc::UnboundedSender<usize> {
        self.slot_sender.clone()
    }

    /// 启动调度者协程
    pub async fn run(self) {
        info!("[SCHEDULER] 调度者协程已启动");

        let mut last_counter_value = 0i64;
        let mut check_interval = tokio::time::interval(tokio::time::Duration::from_millis(1));

        loop {
            check_interval.tick().await;

            // 检查 counter 信号是否有变化
            let current_counter = self.counter.load(Ordering::Acquire);

            if current_counter > last_counter_value {
                debug!(
                    "[SCHEDULER] 检测到新的槽位请求，counter: {} -> {}",
                    last_counter_value, current_counter
                );

                // 尝试处理所有待处理的请求
                let requests_to_process = current_counter - last_counter_value;
                let mut processed = 0;

                for _ in 0..requests_to_process {
                    match self.find_and_reserve_empty_slot().await {
                        Ok(slot_index) => {
                            debug!(
                                "[SCHEDULER] 找到空闲槽位: {}, 状态: EMPTY -> PENDINGWRITE",
                                slot_index
                            );

                            // 发送槽位索引到通道（虽然当前架构中暂时不使用）
                            if let Err(e) = self.slot_sender.send(slot_index) {
                                error!("[SCHEDULER] 发送槽位索引失败: {:?}", e);
                                break;
                            }

                            processed += 1;
                        }
                        Err(e) => {
                            debug!("[SCHEDULER] 未找到可用槽位: {}", e);
                            break; // 没有更多可用槽位，停止处理
                        }
                    }
                }

                // 减少已处理的请求数量
                if processed > 0 {
                    let old_value = self.counter.fetch_sub(processed, Ordering::AcqRel);
                    debug!(
                        "[SCHEDULER] 处理了 {} 个请求，counter: {} -> {}",
                        processed,
                        old_value,
                        self.counter.load(Ordering::Acquire)
                    );
                }

                last_counter_value = self.counter.load(Ordering::Acquire);
            } else if current_counter < last_counter_value {
                // counter 被重置或减少了
                last_counter_value = current_counter;
            }

            // 定期检查是否有槽位从其他状态变为 EMPTY，可以重新分配
            if current_counter > 0 {
                match self.find_and_reserve_empty_slot().await {
                    Ok(slot_index) => {
                        debug!(
                            "[SCHEDULER] 发现新释放的槽位: {}, 状态: EMPTY -> PENDINGWRITE",
                            slot_index
                        );

                        if let Err(e) = self.slot_sender.send(slot_index) {
                            error!("[SCHEDULER] 发送槽位索引失败: {:?}", e);
                            break;
                        }

                        let old_value = self.counter.fetch_sub(1, Ordering::AcqRel);
                        debug!(
                            "[SCHEDULER] 分配释放的槽位 {}, counter: {} -> {}",
                            slot_index,
                            old_value,
                            self.counter.load(Ordering::Acquire)
                        );

                        last_counter_value = self.counter.load(Ordering::Acquire);
                    }
                    Err(_) => {
                        // 没有可用槽位，继续等待
                    }
                }
            }
        }

        warn!("[SCHEDULER] 调度者协程已退出");
    }

    /// 查找并预留空闲槽位
    async fn find_and_reserve_empty_slot(&self) -> Result<usize, &'static str> {
        let slot_index = self
            .queue
            .find_empty_slot()
            .ok_or("没有可用的 EMPTY 槽位")?;

        // 将槽位状态设置为 PENDINGWRITE
        self.queue
            .set_slot_state(slot_index, mi7::SlotState::PENDINGWRITE)
            .map_err(|_| "设置槽位状态失败")?;

        debug!(
            "调度者：找到空闲槽位 {} 并标记为 PENDINGWRITE",
            slot_index
        );
        Ok(slot_index)
    }
}

/// 槽位请求器，用于 handler 获取槽位
pub struct SlotRequester {
    counter: Arc<AtomicI64>,
    slot_receiver: mpsc::UnboundedReceiver<usize>,
}

impl SlotRequester {
    /// 创建新的槽位请求器
    pub fn new(counter: Arc<AtomicI64>, slot_receiver: mpsc::UnboundedReceiver<usize>) -> Self {
        Self {
            counter,
            slot_receiver,
        }
    }

    /// 请求一个槽位索引
    pub async fn request_slot(&mut self) -> Result<usize, &'static str> {
        // 首先尝试从通道获取
        if let Ok(slot_index) = self.slot_receiver.try_recv() {
            debug!("[SLOT_REQUESTER] 直接获取到槽位: {}", slot_index);
            return Ok(slot_index);
        }

        // 如果通道为空，增加计数器并等待
        self.counter.fetch_add(1, Ordering::AcqRel);
        debug!(
            "[SLOT_REQUESTER] 通道为空，counter +1: {}",
            self.counter.load(Ordering::Acquire)
        );

        // 等待槽位可用
        match self.slot_receiver.recv().await {
            Some(slot_index) => {
                debug!("[SLOT_REQUESTER] 等待后获取到槽位: {}", slot_index);
                Ok(slot_index)
            }
            None => {
                error!("[SLOT_REQUESTER] 槽位通道已关闭");
                Err("槽位通道已关闭")
            }
        }
    }
}
