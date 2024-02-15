use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

/// The `ThreadLooper` struct is designed for managing a looping background thread.
///
/// This struct provides functionality to start, manage, and terminate a looping background thread.
/// It is useful for tasks that need to run continuously in the background until explicitly stopped.
///
/// # Fields
/// - `status`: An `Arc<AtomicBool>` indicating whether the looper is active.
/// - `thread`: A `Cell` containing an `Option<thread::JoinHandle<()>>` for the background thread.
/// - `termination`: An `Arc<AtomicBool>` indicating whether the looper should be terminated.
pub struct ThreadLooper {
    status: Arc<AtomicBool>,
    thread: Cell<Option<thread::JoinHandle<()>>>,
    termination: Arc<AtomicBool>,
}

impl ThreadLooper {
    /// Creates a new instance of `ThreadLooper`.
    ///
    /// Initializes the status and termination flags to `false` and sets the thread handle to `None`.
    ///
    /// # Returns
    /// A new instance of `ThreadLooper`.
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

    /// Checks if the looper is currently active.
    pub fn is_active(&self) -> bool {
        self.status.load(LOAD_ORDER)
    }

    /// Starts the looper with the given function.
    ///
    /// If the looper is not already active, this method will start the background thread and run the provided function in a loop.
    ///
    /// # Type Parameters
    /// - `F`: The type of the function to execute in the loop.
    ///
    /// # Arguments
    /// - `function`: The function to be executed in the background thread.
    pub fn start<F>(&self, function: F)
    where
        F: Fn() -> () + Send + 'static,
    {
        if !self.is_active() {
            self.status.store(true, STORE_ORDER);
            self.termination.store(false, STORE_ORDER);

            let thread: thread::JoinHandle<()> = thread::spawn(self.create(function));
            self.thread.set(Some(thread));
        }
    }

    /// Terminates the background thread.
    ///
    /// If the background thread is running, this method will signal it to stop and join the thread, effectively waiting for it to finish execution.
    pub fn terminate(&self) {
        self.termination.store(true, STORE_ORDER);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl ThreadLooper {
    fn create<F>(&self, function: F) -> impl Fn()
    where
        F: Fn() -> () + Send + 'static,
    {
        let status: Arc<AtomicBool> = self.status.clone();
        let termination: Arc<AtomicBool> = self.termination.clone();

        let worker = move || {
            while !termination.load(LOAD_ORDER) {
                function();
            }
            status.store(false, STORE_ORDER);
            termination.store(false, STORE_ORDER);
        };
        worker
    }
}
