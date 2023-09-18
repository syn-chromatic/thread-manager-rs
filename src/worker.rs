use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::channel::AtomicChannel;
use crate::manager::Job;

struct WorkerStatus {
    is_active: Arc<AtomicBool>,
    is_waiting: Arc<AtomicBool>,
    is_busy: Arc<AtomicBool>,
    received_jobs: Arc<AtomicUsize>,
}

impl WorkerStatus {
    fn new() -> Self {
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

    fn set_active_state(&self, state: bool) {
        self.is_active.store(state, Ordering::Release);
    }

    fn set_waiting_state(&self, state: bool) {
        self.is_waiting.store(state, Ordering::Release);
    }

    fn set_busy_state(&self, state: bool) {
        self.is_busy.store(state, Ordering::Release);
    }

    fn add_received_job(&self) {
        self.received_jobs.fetch_add(1, Ordering::Release);
    }

    fn is_active(&self) -> bool {
        let is_active: bool = self.is_active.load(Ordering::Acquire);
        is_active
    }

    fn is_waiting(&self) -> bool {
        let is_waiting: bool = self.is_waiting.load(Ordering::Acquire);
        is_waiting
    }

    fn is_busy(&self) -> bool {
        let is_busy: bool = self.is_busy.load(Ordering::Acquire);
        is_busy
    }

    fn received_jobs(&self) -> usize {
        let received_jobs: usize = self.received_jobs.load(Ordering::Acquire);
        received_jobs
    }
}

struct WorkerSignals {
    join_signal: Arc<AtomicBool>,
    termination_signal: Arc<AtomicBool>,
}

impl WorkerSignals {
    fn new() -> Self {
        let join_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let termination_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        WorkerSignals {
            join_signal,
            termination_signal,
        }
    }

    fn set_join_signal(&self, state: bool) {
        self.join_signal.store(state, Ordering::Release);
    }

    fn set_termination_signal(&self, state: bool) {
        self.termination_signal.store(state, Ordering::Release);
    }

    fn join_signal(&self) -> bool {
        let join_signal: bool = self.join_signal.load(Ordering::Acquire);
        join_signal
    }

    fn termination_signal(&self) -> bool {
        let termination_signal: bool = self.termination_signal.load(Ordering::Acquire);
        termination_signal
    }
}

pub struct ThreadWorker {
    id: usize,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    channel: Arc<AtomicChannel<Job>>,
    active_threads: Arc<AtomicUsize>,
    waiting_threads: Arc<AtomicUsize>,
    busy_threads: Arc<AtomicUsize>,
    status: Arc<WorkerStatus>,
    signals: Arc<WorkerSignals>,
}

impl ThreadWorker {
    pub fn new(
        id: usize,
        channel: Arc<AtomicChannel<Job>>,
        active_threads: Arc<AtomicUsize>,
        waiting_threads: Arc<AtomicUsize>,
        busy_threads: Arc<AtomicUsize>,
    ) -> Self {
        let thread: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
        let status: Arc<WorkerStatus> = Arc::new(WorkerStatus::new());
        let signals: Arc<WorkerSignals> = Arc::new(WorkerSignals::new());

        ThreadWorker {
            id,
            thread,
            channel,
            active_threads,
            waiting_threads,
            busy_threads,
            status,
            signals,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn start(&self) {
        if !self.is_active() {
            if let Ok(mut thread_guard) = self.thread.lock() {
                if let Some(existing_thread) = thread_guard.take() {
                    let _ = existing_thread.join();
                    let pending_count: usize = self.channel.get_pending_count();
                    let waiting_threads: usize = self.waiting_threads.load(Ordering::Acquire);
                    if waiting_threads > pending_count {
                        return;
                    }
                }

                let active_threads: Arc<AtomicUsize> = self.active_threads.clone();
                Self::set_worker_active(&active_threads, &self.status);

                let worker_loop = self.create_worker_loop();
                let thread: thread::JoinHandle<()> = thread::spawn(worker_loop);
                *thread_guard = Some(thread);
            }
        }
    }

    pub fn send<F>(&self, function: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(function);
        self.channel
            .send(job)
            .expect(&format!("Failed to send job to Worker [{}]", self.id));
        self.start();
    }

    pub fn join(&self) {
        if let Ok(mut thread_option) = self.thread.lock() {
            if let Some(thread) = thread_option.take() {
                let _ = thread.join();
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        if let Ok(thread_option) = self.thread.lock() {
            if let Some(thread) = thread_option.as_ref() {
                let is_finished: bool = thread.is_finished();
                return is_finished;
            }
        }
        false
    }

    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    pub fn is_busy(&self) -> bool {
        self.status.is_busy()
    }

    pub fn is_waiting(&self) -> bool {
        self.status.is_waiting()
    }

    pub fn get_received_jobs(&self) -> usize {
        self.status.received_jobs()
    }

    pub fn send_join_signal(&self) {
        self.signals.set_join_signal(true);
    }

    pub fn send_termination_signal(&self) {
        self.signals.set_termination_signal(true);
        self.send_channel_release();
    }

    pub fn send_channel_release(&self) {
        let closure: Job = Box::new(move || {});
        self.channel
            .send_release(Box::new(closure))
            .expect(&format!("Failed to release Worker [{}]", self.id));
    }
}

impl ThreadWorker {
    fn set_worker_active(active_threads: &Arc<AtomicUsize>, status: &Arc<WorkerStatus>) {
        status.set_active_state(true);
        active_threads.fetch_add(1, Ordering::Release);
    }

    fn unset_worker_active(active_threads: &Arc<AtomicUsize>, status: &Arc<WorkerStatus>) {
        status.set_active_state(false);
        active_threads.fetch_sub(1, Ordering::Release);
    }

    fn set_worker_waiting(waiting_threads: &Arc<AtomicUsize>, status: &Arc<WorkerStatus>) {
        status.set_waiting_state(true);
        waiting_threads.fetch_add(1, Ordering::Release);
    }

    fn unset_worker_waiting(waiting_threads: &Arc<AtomicUsize>, status: &Arc<WorkerStatus>) {
        status.set_waiting_state(false);
        waiting_threads.fetch_sub(1, Ordering::Release);
    }

    fn set_worker_busy(busy_threads: &Arc<AtomicUsize>, status: &Arc<WorkerStatus>) {
        status.set_busy_state(true);
        busy_threads.fetch_add(1, Ordering::Release);
    }

    fn unset_worker_busy(busy_threads: &Arc<AtomicUsize>, status: &Arc<WorkerStatus>) {
        status.set_busy_state(false);
        busy_threads.fetch_sub(1, Ordering::Release);
    }

    fn create_worker_loop(&self) -> impl Fn() {
        let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
        let active_threads: Arc<AtomicUsize> = self.active_threads.clone();
        let waiting_threads: Arc<AtomicUsize> = self.waiting_threads.clone();
        let busy_threads: Arc<AtomicUsize> = self.busy_threads.clone();
        let status: Arc<WorkerStatus> = self.status.clone();
        let signals: Arc<WorkerSignals> = self.signals.clone();

        let worker_loop = move || {
            while !signals.termination_signal() {
                if signals.join_signal() {
                    let pending_jobs: usize = channel.get_pending_count();
                    if pending_jobs == 0 {
                        break;
                    }
                }

                Self::set_worker_waiting(&waiting_threads, &status);
                let recv = channel.recv();
                Self::unset_worker_waiting(&waiting_threads, &status);
                if let Ok((job, kind)) = recv {
                    status.add_received_job();
                    Self::set_worker_busy(&busy_threads, &status);
                    job();
                    Self::unset_worker_busy(&busy_threads, &status);
                    channel.conclude(kind);
                }
            }

            Self::unset_worker_active(&active_threads, &status);
            signals.set_termination_signal(false);
        };
        worker_loop
    }
}
