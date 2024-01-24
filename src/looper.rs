use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use crate::status::WorkerSignals;

use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

pub struct ThreadLooper {
    status: Arc<AtomicBool>,
    signals: Arc<WorkerSignals>,
    thread: Cell<Option<thread::JoinHandle<()>>>,
}

impl ThreadLooper {
    pub fn new() -> Self {
        let status: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let signals: Arc<WorkerSignals> = Arc::new(WorkerSignals::new());
        let thread: Cell<Option<thread::JoinHandle<()>>> = Cell::new(None);

        Self {
            status,
            signals,
            thread,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status.load(LOAD_ORDER)
    }

    pub fn start<F>(&self, function: F)
    where
        F: Fn() -> () + Send + 'static,
    {
        if !self.is_active() {
            self.status.store(true, STORE_ORDER);
            let thread: thread::JoinHandle<()> = thread::spawn(self.create(function));
            self.thread.set(Some(thread));
        }
    }

    pub fn terminate(&self) {
        self.signals.set_termination_signal(true);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl ThreadLooper {
    fn looper<F>(function: &F, signals: &Arc<WorkerSignals>)
    where
        F: Fn() -> () + Send + 'static,
    {
        while !signals.termination_signal() {
            function();
        }
    }

    fn create<F>(&self, function: F) -> impl Fn()
    where
        F: Fn() -> () + Send + 'static,
    {
        let status: Arc<AtomicBool> = self.status.clone();
        let signals: Arc<WorkerSignals> = self.signals.clone();

        let worker = move || {
            Self::looper(&function, &signals);
            status.store(false, STORE_ORDER);
            signals.set_termination_signal(false);
        };
        worker
    }
}
