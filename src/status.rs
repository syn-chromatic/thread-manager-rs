use std::sync::Arc;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;

use crate::order::FETCH_ORDER;
use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

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
        Self {
            active_threads,
            waiting_threads,
            busy_threads,
        }
    }

    pub fn active_threads(&self) -> usize {
        self.active_threads.load(LOAD_ORDER)
    }

    pub fn waiting_threads(&self) -> usize {
        self.waiting_threads.load(LOAD_ORDER)
    }

    pub fn busy_threads(&self) -> usize {
        self.busy_threads.load(LOAD_ORDER)
    }

    pub fn set_active(&self, state: bool) {
        match state {
            true => self.active_threads.fetch_add(1, FETCH_ORDER),
            false => self.active_threads.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_waiting(&self, state: bool) {
        match state {
            true => self.waiting_threads.fetch_add(1, FETCH_ORDER),
            false => self.waiting_threads.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_busy(&self, state: bool) {
        match state {
            true => self.busy_threads.fetch_add(1, FETCH_ORDER),
            false => self.busy_threads.fetch_sub(1, FETCH_ORDER),
        };
    }
}

pub struct WorkerStatus {
    active: Arc<AtomicBool>,
    waiting: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
}

impl WorkerStatus {
    pub fn new() -> Self {
        let active: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let waiting: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let busy: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        Self {
            active,
            waiting,
            busy,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(LOAD_ORDER)
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting.load(LOAD_ORDER)
    }

    pub fn is_busy(&self) -> bool {
        self.busy.load(LOAD_ORDER)
    }

    pub fn set_active(&self, state: bool) {
        self.active.store(state, STORE_ORDER);
    }

    pub fn set_waiting(&self, state: bool) {
        self.waiting.store(state, STORE_ORDER);
    }

    pub fn set_busy(&self, state: bool) {
        self.busy.store(state, STORE_ORDER);
    }
}

pub struct ChannelStatus {
    sent: Arc<AtomicUsize>,
    sending: Arc<AtomicUsize>,
    received: Arc<AtomicUsize>,
    receiving: Arc<AtomicUsize>,
    concluded: Arc<AtomicUsize>,
}

impl ChannelStatus {
    pub fn new() -> Self {
        let sent: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let sending: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let received: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let receiving: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let concluded: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        Self {
            sent,
            sending,
            received,
            receiving,
            concluded,
        }
    }

    pub fn available(&self) -> usize {
        let sent_count: usize = self.sent();
        let received_count: usize = self.received();
        sent_count - received_count
    }

    pub fn pending(&self) -> usize {
        let sent_count: usize = self.sent();
        let sending_count: usize = self.sending();
        let received_count: usize = self.received();
        (sent_count + sending_count) - received_count
    }

    pub fn sent(&self) -> usize {
        self.sent.load(LOAD_ORDER)
    }

    pub fn sending(&self) -> usize {
        self.sending.load(LOAD_ORDER)
    }

    pub fn received(&self) -> usize {
        self.received.load(LOAD_ORDER)
    }

    pub fn receiving(&self) -> usize {
        self.receiving.load(LOAD_ORDER)
    }

    pub fn concluded(&self) -> usize {
        self.concluded.load(LOAD_ORDER)
    }

    pub fn add_sent(&self) {
        self.sent.fetch_add(1, FETCH_ORDER);
    }

    pub fn add_sending(&self) {
        self.sending.fetch_add(1, FETCH_ORDER);
    }

    pub fn add_received(&self) {
        self.received.fetch_add(1, FETCH_ORDER);
    }

    pub fn add_receiving(&self) {
        self.receiving.fetch_add(1, FETCH_ORDER);
    }
    pub fn add_concluded(&self) {
        self.concluded.fetch_add(1, FETCH_ORDER);
    }

    pub fn sub_sending(&self) {
        self.sending.fetch_sub(1, FETCH_ORDER);
    }

    pub fn sub_receiving(&self) {
        self.receiving.fetch_sub(1, FETCH_ORDER);
    }
}
