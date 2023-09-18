use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

const LOAD_ORDER: Ordering = Ordering::Acquire;
const STORE_ORDER: Ordering = Ordering::Release;
const FETCH_ORDER: Ordering = Ordering::Release;

pub struct ManagerStatus {
    active_threads: Arc<AtomicUsize>,
    waiting_threads: Arc<AtomicUsize>,
    busy_threads: Arc<AtomicUsize>,
}

impl ManagerStatus {
    pub fn new() -> Self {
        let active_threads: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let waiting_threads: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let busy_threads: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        ManagerStatus {
            active_threads,
            waiting_threads,
            busy_threads,
        }
    }

    pub fn get_active_threads(&self) -> usize {
        let active_threads: usize = self.active_threads.load(LOAD_ORDER);
        active_threads
    }

    pub fn get_waiting_threads(&self) -> usize {
        let waiting_threads: usize = self.waiting_threads.load(LOAD_ORDER);
        waiting_threads
    }

    pub fn get_busy_threads(&self) -> usize {
        let busy_threads: usize = self.busy_threads.load(LOAD_ORDER);
        busy_threads
    }

    pub fn add_active_threads(&self) {
        self.active_threads.fetch_add(1, FETCH_ORDER);
    }

    pub fn sub_active_threads(&self) {
        self.active_threads.fetch_sub(1, FETCH_ORDER);
    }

    pub fn add_waiting_threads(&self) {
        self.waiting_threads.fetch_add(1, FETCH_ORDER);
    }

    pub fn sub_waiting_threads(&self) {
        self.waiting_threads.fetch_sub(1, FETCH_ORDER);
    }

    pub fn add_busy_threads(&self) {
        self.busy_threads.fetch_add(1, FETCH_ORDER);
    }

    pub fn sub_busy_threads(&self) {
        self.busy_threads.fetch_sub(1, FETCH_ORDER);
    }
}

pub struct WorkerStatus {
    is_active: Arc<AtomicBool>,
    is_waiting: Arc<AtomicBool>,
    is_busy: Arc<AtomicBool>,
    received_jobs: Arc<AtomicUsize>,
}

impl WorkerStatus {
    pub fn new() -> Self {
        let is_active: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let is_waiting: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let is_busy: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let received_jobs: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        WorkerStatus {
            is_active,
            is_waiting,
            is_busy,
            received_jobs,
        }
    }

    pub fn get_is_active(&self) -> bool {
        let is_active: bool = self.is_active.load(LOAD_ORDER);
        is_active
    }

    pub fn get_is_waiting(&self) -> bool {
        let is_waiting: bool = self.is_waiting.load(LOAD_ORDER);
        is_waiting
    }

    pub fn get_is_busy(&self) -> bool {
        let is_busy: bool = self.is_busy.load(LOAD_ORDER);
        is_busy
    }

    pub fn get_received_jobs(&self) -> usize {
        let received_jobs: usize = self.received_jobs.load(LOAD_ORDER);
        received_jobs
    }

    pub fn set_active_state(&self, state: bool) {
        self.is_active.store(state, STORE_ORDER);
    }

    pub fn set_waiting_state(&self, state: bool) {
        self.is_waiting.store(state, STORE_ORDER);
    }

    pub fn set_busy_state(&self, state: bool) {
        self.is_busy.store(state, STORE_ORDER);
    }

    pub fn add_received_job(&self) {
        self.received_jobs.fetch_add(1, FETCH_ORDER);
    }
}

pub struct ChannelStatus {
    sent_count: Arc<AtomicUsize>,
    sending_count: Arc<AtomicUsize>,
    received_count: Arc<AtomicUsize>,
    receiving_count: Arc<AtomicUsize>,
    concluded_count: Arc<AtomicUsize>,
}

impl ChannelStatus {
    pub fn new() -> Self {
        let sent_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let sending_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let received_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let receiving_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let concluded_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        ChannelStatus {
            sent_count,
            sending_count,
            received_count,
            receiving_count,
            concluded_count,
        }
    }

    pub fn get_sent_count(&self) -> usize {
        let sent_count: usize = self.sent_count.load(LOAD_ORDER);
        sent_count
    }

    pub fn get_sending_count(&self) -> usize {
        let sending_count: usize = self.sending_count.load(LOAD_ORDER);
        sending_count
    }

    pub fn get_received_count(&self) -> usize {
        let received_count: usize = self.received_count.load(LOAD_ORDER);
        received_count
    }

    pub fn get_receiving_count(&self) -> usize {
        let receiving_count: usize = self.receiving_count.load(LOAD_ORDER);
        receiving_count
    }

    pub fn get_concluded_count(&self) -> usize {
        let concluded_count: usize = self.concluded_count.load(LOAD_ORDER);
        concluded_count
    }

    pub fn add_sent_count(&self) {
        self.sent_count.fetch_add(1, FETCH_ORDER);
    }

    pub fn add_sending_count(&self) {
        self.sending_count.fetch_add(1, FETCH_ORDER);
    }

    pub fn sub_sending_count(&self) {
        self.sending_count.fetch_sub(1, FETCH_ORDER);
    }

    pub fn add_received_count(&self) {
        self.received_count.fetch_add(1, FETCH_ORDER);
    }

    pub fn add_receiving_count(&self) {
        self.receiving_count.fetch_add(1, FETCH_ORDER);
    }

    pub fn sub_receiving_count(&self) {
        self.receiving_count.fetch_sub(1, FETCH_ORDER);
    }

    pub fn add_concluded_count(&self) {
        self.concluded_count.fetch_add(1, FETCH_ORDER);
    }
}
