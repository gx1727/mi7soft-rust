use mi7::{CrossProcessPipe, shared_slot::SlotState};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub struct Listener {
    pipe: CrossProcessPipe<100, 4096>,
    // slot_state: Arc<SlotState>,
    // tx: mpsc::Sender<()>,
}

impl Listener {
    pub fn new(pipe: CrossProcessPipe<100, 4096>) -> Self {
        Self { pipe }
    }

    pub async fn run(self) {
        println!("listener started")
    }
}
