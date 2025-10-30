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
}
