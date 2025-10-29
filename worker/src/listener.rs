use mi7::{CrossProcessPipe, shared_slot::SlotState};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub struct Listener {
    pipe: Arc<CrossProcessPipe<100, 4096>>,
    // slot_state: Arc<SlotState>,
    // tx: mpsc::Sender<()>,
}

impl Listener {
    pub fn new(
        pipe: Arc<CrossProcessPipe<100, 4096>>
    ) -> Self {
        Self {
            pipe
        }
    }
}
