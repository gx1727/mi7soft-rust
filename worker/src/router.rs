use crate::listener::Listener;
use anyhow::Result;
use mi7::pipe::DynamicPipe;
use mi7::shared_slot::SlotState;
use std::sync::Arc;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info};

pub struct Router {
    worker_id: String,
    pipe: Arc<Box<dyn DynamicPipe>>,
}

impl Router {
    pub fn new(worker_id: String, pipe: Arc<Box<dyn DynamicPipe>>) -> Router {
        Self { worker_id, pipe }
    }

    pub async fn run(&self) -> Result<()> {
        // 设置槽位状态为处理中
        // if let Err(_) = self.pipe.set_slot_state(slot_index, SlotState::INPROGRESS) {
        //     error!("Listener {} 设置槽位状态失败", self.worker_id);
        //     continue;
        // }
        //
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
        //         "Listener {} 收到任务 flag={}: {}",
        //         self.worker_id,
        //         message.flag,
        //         String::from_utf8_lossy(&message.data)
        //     );
        Ok(())
    }
}
