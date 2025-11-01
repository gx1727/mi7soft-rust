/**
 * 处理者
 */
use async_channel::Receiver;
use mi7::pipe::DynamicPipe;
use std::sync::Arc;
use tracing::{error, info};

pub struct Operator {
    rx: Receiver<usize>,
    pipe: Arc<Box<dyn DynamicPipe>>,
}

impl Operator {
    pub fn new(rx: Receiver<usize>, pipe: Arc<Box<dyn DynamicPipe>>) -> Operator {
        Operator { rx, pipe }
    }

    pub async fn run(&self, i: i32) -> anyhow::Result<()> {
        // if let Err(_) = self.pipe.set_slot_state(slot_index, SlotState::INPROGRESS) {
        //     error!("Listener {} 设置槽位状态失败", self.worker_id);
        //     continue;
        // }

        loop {
            info!("消费者 {} 开始等待接收消息...", i);
            match self.rx.recv().await {
                Ok(slot_index) => {
                    info!(
                        "消费者 {} 接收到消息: {} (时间戳: {:?})",
                        i,
                        slot_index,
                        std::time::SystemTime::now()
                    );

                    // // 接收消息
                    let message = match self.pipe.receive(slot_index) {
                        Ok(msg) => msg,
                        Err(_) => {
                            error!("Listener {} 读取消息失败", slot_index);

                            continue;
                        }
                    };
                    info!(
                        "Listener {} 收到任务 flag={}: {}",
                        slot_index,
                        message.flag,
                        String::from_utf8_lossy(&message.data)
                    );

                    // 这里可以添加实际的消息处理逻辑
                    // 比如调用 router 处理消息
                }
                Err(e) => {
                    error!("消费者 {} 接收消息失败: {:?}", i, e);
                    break; // 通道关闭时退出循环
                }
            }

            info!("消费者 {} 开始等待接收消息... end", i);
        }

        // // 接收消息
        // let message = match self.pipe.receive(slot_index) {
        //     Ok(msg) => msg,
        //     Err(_) => {
        //         error!("Listener {} 读取消息失败", self.worker_id);
        //
        //         continue;
        //     }
        // };
        // info!(
        //     "Listener {} 收到任务 flag={}: {}",
        //     self.worker_id,
        //     message.flag,
        //     String::from_utf8_lossy(&message.data)
        // );

        Ok(())
    }
}
