use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct WorkerSignals {
    join_signal: Arc<AtomicBool>,
    termination_signal: Arc<AtomicBool>,
}

impl WorkerSignals {
    pub fn new() -> Self {
        let join_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let termination_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        WorkerSignals {
            join_signal,
            termination_signal,
        }
    }

    pub fn get_join_signal(&self) -> bool {
        let join_signal: bool = self.join_signal.load(Ordering::Acquire);
        join_signal
    }

    pub fn get_termination_signal(&self) -> bool {
        let termination_signal: bool = self.termination_signal.load(Ordering::Acquire);
        termination_signal
    }

    pub fn set_join_signal(&self, state: bool) {
        self.join_signal.store(state, Ordering::Release);
    }

    pub fn set_termination_signal(&self, state: bool) {
        self.termination_signal.store(state, Ordering::Release);
    }
}
