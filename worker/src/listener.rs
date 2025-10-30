use mi7::pipe::DynamicPipe;
use mi7::shared_slot::SlotState;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

pub struct Listener {
    worker_id: String,
}

impl Listener {
    pub fn new(worker_id: String) -> Listener {
        Self { worker_id }
    }

    pub async fn run(&self, pipe: Box<dyn DynamicPipe>) {
        info!("Listener {} 启动", self.worker_id);

        let mut consecutive_empty = 0;
        let mut processed_count = 0;

        loop {
            // 尝试获取任务
            let slot_index_result = pipe.fetch();
            let slot_index = match slot_index_result {
                Ok(index) => index,
                Err(_) => {
                    // 队列为空，无法获取消息索引
                    consecutive_empty += 1;

                    if consecutive_empty == 1 {
                        info!("Listener {} 等待新任务...", self.worker_id);
                    }

                    // 如果连续多次没有任务，考虑退出
                    if consecutive_empty > 120 {
                        // 120次检查没有任务（约1分钟）
                        info!("Listener {} 长时间无任务，准备退出", self.worker_id);
                        break;
                    }
                    // 短暂等待后重试
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }
            };

            // 设置槽位状态为处理中
            let set_state_result = pipe.set_slot_state(slot_index, SlotState::INPROGRESS);
            if set_state_result.is_err() {
                error!("Listener {} 设置槽位状态失败", self.worker_id);
                continue;
            }

            // 接收消息
            let receive_result = pipe.receive(slot_index);
            let message = match receive_result {
                Ok(msg) => msg,
                Err(_) => {
                    error!("Listener {} 读取消息失败", self.worker_id);
                    consecutive_empty += 1;
                    continue;
                }
            };

            // 重置连续空计数
            consecutive_empty = 0;
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
            let status = pipe.status();
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
