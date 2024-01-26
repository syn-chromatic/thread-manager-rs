use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

pub struct ThreadLooper {
    status: Arc<AtomicBool>,
    thread: Cell<Option<thread::JoinHandle<()>>>,
    termination: Arc<AtomicBool>,
}

impl ThreadLooper {
    pub fn new() -> Self {
        let status: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let thread: Cell<Option<thread::JoinHandle<()>>> = Cell::new(None);
        let termination: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        Self {
            status,
            thread,
            termination,
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
        self.termination.store(true, STORE_ORDER);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl ThreadLooper {
    fn looper<F>(function: &F, termination: &Arc<AtomicBool>)
    where
        F: Fn() -> () + Send + 'static,
    {
        while !termination.load(LOAD_ORDER) {
            function();
        }
    }

    fn create<F>(&self, function: F) -> impl Fn()
    where
        F: Fn() -> () + Send + 'static,
    {
        let status: Arc<AtomicBool> = self.status.clone();
        let termination: Arc<AtomicBool> = self.termination.clone();

        let worker = move || {
            Self::looper(&function, &termination);
            status.store(false, STORE_ORDER);
            termination.store(false, STORE_ORDER);
        };
        worker
    }
}
