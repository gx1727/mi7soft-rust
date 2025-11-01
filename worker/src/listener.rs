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

        // self.tx.send(1).await.expect("Listener {} 发送消息失败");
        // info!("这时应该打印 '消费者 接收到消息: 1' ...");
        // tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        // self.tx.send(2).await.expect("Listener {} 发送消息失败");
        // info!("这时应该打印 '消费者 接收到消息: 2' ...");
        // tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let mut processed_count = 0;

        loop {
            // 尝试获取任务
            info!("Listener {} 尝试获取任务", self.worker_id);
            let slot_index = match self.pipe.fetch() {
                Ok(index) => index,
                Err(_) => {
                    // fetch中已有 短暂等待
                    // 重试
                    info!("重试");
                    continue;
                }
            };

            info!("Listener 获取任务 {} ", slot_index);

            match tokio::time::timeout(std::time::Duration::from_secs(30), self.tx.send(slot_index))
                .await
            {
                Ok(Ok(())) => {
                    // 发送成功
                    info!("Listener 发送任务 {} ", slot_index);
                    // 主动让出 CPU 时间，让消费者有机会处理消息
                    tokio::task::yield_now().await;
                }
                Ok(Err(e)) => {
                    // 通道发送错误
                    eprintln!("Failed to send slot index: {:?}", e);
                }
                Err(_) => {
                    // 超时
                    eprintln!("Timeout while sending slot index");
                }
            }
        }

        info!(
            "Listener {} 退出，共处理 {} 个任务",
            self.worker_id, processed_count
        );
    }
}
