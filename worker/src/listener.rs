use mi7::CrossProcessPipe;

pub struct Listener {
    pipe: CrossProcessPipe,
    // slot_state: Arc<SlotState>,
    // tx: mpsc::Sender<()>,
}

impl Listener {
    pub fn new(pipe: CrossProcessPipe) -> Self {
        Self { pipe }
    }

    pub async fn run(self) {
        println!("listener started")
    }
}
