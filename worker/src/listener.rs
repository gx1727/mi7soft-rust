use anyhow::Result;
use async_channel::Sender;
use mi7::pipe::DynamicPipe;
use mi7::shared_slot::SlotState;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

pub struct Listener {
    worker_id: String,
    pipe: Arc<Box<dyn DynamicPipe>>,
    tx: Sender<usize>,
}

impl Listener {
    pub fn new(worker_id: String, pipe: Arc<Box<dyn DynamicPipe>>, tx: Sender<usize>) -> Listener {
        Self {
            worker_id,
            pipe,
            tx,
        }
    }

    pub async fn run(&self) {
        info!("Listener {} 启动", self.worker_id);

        let mut processed_count = 0;

        loop {
            // 尝试获取任务
            info!("Listener {} 尝试获取任务", self.worker_id);
            let slot_index = match self.pipe.fetch() {
                Ok(index) => index,
                Err(_) => {
                    // fetch中已有 短暂等待
                    // 重试
                    continue;
                }
            };

            info!("Listener 获取任务 {} ", slot_index);

            self.tx
                .send(slot_index)
                .await
                .expect("Listener {} 发送消息失败");
            info!("Listener 发送任务 {} ", slot_index);
            continue;

            // 设置槽位状态为处理中
            if let Err(_) = self.pipe.set_slot_state(slot_index, SlotState::INPROGRESS) {
                error!("Listener {} 设置槽位状态失败", self.worker_id);
                continue;
            }

            // 接收消息
            let message = match self.pipe.receive(slot_index) {
                Ok(msg) => msg,
                Err(_) => {
                    error!("Listener {} 读取消息失败", self.worker_id);

                    continue;
                }
            };

            // 重置连续空计数
            processed_count += 1;

            info!(
                "Listener {} 收到任务 flag={}: {}",
                self.worker_id,
                message.flag,
                String::from_utf8_lossy(&message.data)
            );

            // 模拟任务处理时间
            let processing_time = Duration::from_millis(
                100 + (message.timestamp % 5) * 200, // 100-900ms的随机处理时间
            );
            sleep(processing_time).await;

            info!(
                "Listener {} 完成任务 flag={} (耗时: {:?})",
                self.worker_id, message.flag, processing_time
            );

            // 显示队列状态
            let status = self.pipe.status();
            debug!(
                "Listener {} 队列状态: {}/{} 消息剩余",
                self.worker_id, status.ready_count, status.capacity
            );
        }

        info!(
            "Listener {} 退出，共处理 {} 个任务",
            self.worker_id, processed_count
        );
    }
}
