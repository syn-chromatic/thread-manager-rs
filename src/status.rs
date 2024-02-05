use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;

use crate::order::FETCH_ORDER;
use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

pub enum Operation {
    Add,
    Sub,
}

impl Into<Operation> for bool {
    fn into(self) -> Operation {
        match self {
            true => Operation::Add,
            false => Operation::Sub,
        }
    }
}

pub struct ManagerStatus {
    active_threads: AtomicUsize,
    waiting_threads: AtomicUsize,
    busy_threads: AtomicUsize,
}

impl ManagerStatus {
    pub fn new() -> Self {
        let active_threads: AtomicUsize = AtomicUsize::new(0);
        let waiting_threads: AtomicUsize = AtomicUsize::new(0);
        let busy_threads: AtomicUsize = AtomicUsize::new(0);
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

    pub fn set_active<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.active_threads.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.active_threads.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_waiting<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.waiting_threads.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.waiting_threads.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_busy<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.busy_threads.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.busy_threads.fetch_sub(1, FETCH_ORDER),
        };
    }
}

pub struct WorkerStatus {
    active: AtomicBool,
    waiting: AtomicBool,
    busy: AtomicBool,
    received: AtomicUsize,
}

impl WorkerStatus {
    pub fn new() -> Self {
        let active: AtomicBool = AtomicBool::new(false);
        let waiting: AtomicBool = AtomicBool::new(false);
        let busy: AtomicBool = AtomicBool::new(false);
        let received: AtomicUsize = AtomicUsize::new(0);

        Self {
            active,
            waiting,
            busy,
            received,
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

    pub fn received(&self) -> usize {
        self.received.load(LOAD_ORDER)
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

    pub fn add_received(&self) {
        self.received.fetch_add(1, STORE_ORDER);
    }
}

pub struct ChannelStatus {
    sent: AtomicUsize,
    sending: AtomicUsize,
    received: AtomicUsize,
    receiving: AtomicUsize,
    concluded: AtomicUsize,
}

impl ChannelStatus {
    pub fn new() -> Self {
        let sent: AtomicUsize = AtomicUsize::new(0);
        let sending: AtomicUsize = AtomicUsize::new(0);
        let received: AtomicUsize = AtomicUsize::new(0);
        let receiving: AtomicUsize = AtomicUsize::new(0);
        let concluded: AtomicUsize = AtomicUsize::new(0);

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

    pub fn set_sent<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.sent.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.sent.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_sending<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.sending.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.sending.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_received<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.received.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.received.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_receiving<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.receiving.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.receiving.fetch_sub(1, FETCH_ORDER),
        };
    }

    pub fn set_concluded<P>(&self, op: P)
    where
        P: Into<Operation>,
    {
        match op.into() {
            Operation::Add => self.concluded.fetch_add(1, FETCH_ORDER),
            Operation::Sub => self.concluded.fetch_sub(1, FETCH_ORDER),
        };
    }
}
