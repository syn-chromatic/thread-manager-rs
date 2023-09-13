use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::channel::AtomicChannel;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadManager {
    thread_size: usize,
    channel: Arc<AtomicChannel<Job>>,
    workers: Vec<ThreadWorker>,
    dispatch_worker: AtomicUsize,
}

impl ThreadManager {
    pub fn new(thread_size: usize) -> Self {
        let channel: Arc<AtomicChannel<Job>> = Arc::new(AtomicChannel::new());
        let workers: Vec<ThreadWorker> = Self::create_workers(thread_size, channel.clone());
        let dispatch_worker: AtomicUsize = AtomicUsize::new(0);

        ThreadManager {
            thread_size,
            channel,
            workers,
            dispatch_worker,
        }
    }

    pub fn execute<F>(&self, function: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let dispatch_worker: usize = self.dispatch_worker.load(Ordering::Acquire);
        let worker: &ThreadWorker = &self.workers[dispatch_worker];
        worker.send(function);
        self.update_dispatch_worker(dispatch_worker);
    }

    pub fn join(&self) {
        for worker in self.workers.iter() {
            worker.send_join_signal();
        }

        for worker in self.workers.iter() {
            worker.join();
        }
    }

    pub fn set_thread_size(&mut self, thread_size: usize) {
        if thread_size > self.workers.len() {
            let additional_threads: usize = thread_size - self.workers.len();
            let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
            let workers: Vec<ThreadWorker> = Self::create_workers(additional_threads, channel);
            self.workers.extend(workers);
        } else if thread_size < self.workers.len() {
            let split_workers: Vec<ThreadWorker> = self.workers.split_off(thread_size);
            for worker in split_workers.iter() {
                worker.send_termination_signal();
            }
        }
    }

    pub fn send_join_signals(&self) {
        for worker in self.workers.iter() {
            worker.send_join_signal();
        }
    }

    pub fn has_finished(&self) -> bool {
        for worker in self.workers.iter() {
            if worker.is_active() {
                return false;
            }
        }
        true
    }

    pub fn get_active_threads(&self) -> usize {
        let mut active_threads: usize = 0;
        for worker in self.workers.iter() {
            if worker.is_active() {
                active_threads += 1;
            }
        }
        active_threads
    }

    pub fn get_busy_threads(&self) -> usize {
        let mut busy_threads: usize = 0;
        for worker in self.workers.iter() {
            if worker.is_busy() {
                busy_threads += 1;
            }
        }
        busy_threads
    }

    pub fn get_job_queue(&self) -> usize {
        let job_queue: usize = self.channel.get_pending_count();
        job_queue
    }

    pub fn terminate_all(&self) {
        for worker in self.workers.iter() {
            worker.send_termination_signal();
        }

        for worker in self.workers.iter() {
            worker.join();
        }

        self.clear_job_queue();
    }

    pub fn clear_job_queue(&self) {
        self.channel.clear_receiver();
    }
}

impl ThreadManager {
    fn create_workers(thread_size: usize, channel: Arc<AtomicChannel<Job>>) -> Vec<ThreadWorker> {
        let mut workers: Vec<ThreadWorker> = Vec::with_capacity(thread_size);

        for id in 0..thread_size {
            let worker: ThreadWorker = ThreadWorker::new(id, channel.clone());
            worker.start();
            workers.push(worker);
        }
        workers
    }

    fn update_dispatch_worker(&self, dispatch_worker: usize) {
        let next_dispatch: usize = if dispatch_worker >= (self.thread_size - 1) {
            0
        } else {
            dispatch_worker + 1
        };
        self.dispatch_worker.store(next_dispatch, Ordering::Release);
    }
}

struct ThreadWorker {
    id: usize,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    channel: Arc<AtomicChannel<Job>>,
    is_active: Arc<AtomicBool>,
    is_busy: Arc<AtomicBool>,
    join_signal: Arc<AtomicBool>,
    termination_signal: Arc<AtomicBool>,
}

impl ThreadWorker {
    fn new(id: usize, channel: Arc<AtomicChannel<Job>>) -> Self {
        let thread: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
        let is_active: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let is_busy: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let join_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let termination_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        ThreadWorker {
            id,
            thread,
            channel,
            is_active,
            is_busy,
            join_signal,
            termination_signal,
        }
    }

    fn id(&self) -> usize {
        self.id
    }

    fn start(&self) {
        if !self.is_active() {
            self.is_active.store(true, Ordering::Release);
            let worker_loop = self.create_worker_loop();
            let thread: thread::JoinHandle<()> = thread::spawn(worker_loop);
            if let Ok(mut thread_guard) = self.thread.lock() {
                *thread_guard = Some(thread);
            }
        }
    }

    fn send<F>(&self, function: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.start();
        let job: Job = Box::new(function);
        self.channel
            .send(job)
            .expect(&format!("Failed to send job to Worker [{}]", self.id));
    }

    fn join(&self) {
        if let Ok(mut thread_option) = self.thread.lock() {
            if let Some(thread) = thread_option.take() {
                let _ = thread.join();
            }
        }
    }

    fn is_active(&self) -> bool {
        let is_active: bool = self.is_active.load(Ordering::Acquire);
        is_active
    }

    fn is_busy(&self) -> bool {
        let is_busy: bool = self.is_busy.load(Ordering::Acquire);
        is_busy
    }

    fn is_finished(&self) -> bool {
        if let Ok(thread_option) = self.thread.lock() {
            if let Some(thread) = thread_option.as_ref() {
                let is_finished: bool = thread.is_finished();
                return is_finished;
            }
        }
        false
    }

    fn send_join_signal(&self) {
        self.join_signal.store(true, Ordering::Release);
    }

    fn send_termination_signal(&self) {
        self.termination_signal.store(true, Ordering::Release);
        let closure: Job = Box::new(|| {});
        self.channel.send(Box::new(closure)).expect(&format!(
            "Failed to send termination to Worker [{}]",
            self.id
        ));
    }

    fn create_worker_loop(&self) -> impl Fn() {
        let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
        let is_active: Arc<AtomicBool> = self.is_active.clone();
        let is_busy: Arc<AtomicBool> = self.is_busy.clone();
        let join_signal: Arc<AtomicBool> = self.join_signal.clone();
        let termination_signal: Arc<AtomicBool> = self.termination_signal.clone();

        let worker_loop = move || {
            let recv_timeout: Duration = Duration::from_micros(1);
            while !termination_signal.load(Ordering::Acquire) {
                if join_signal.load(Ordering::Acquire) {
                    if channel.get_pending_count() == 0 && channel.get_receive_count() > 0 {
                        break;
                    }
                }

                if let Ok(job) = channel.recv_timeout(recv_timeout) {
                    is_busy.store(true, Ordering::Release);
                    job();
                    is_busy.store(false, Ordering::Release);
                }
            }
            is_active.store(false, Ordering::Release);
            termination_signal.store(false, Ordering::Release);
        };
        worker_loop
    }
}
