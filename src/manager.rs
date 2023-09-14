use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::channel::AtomicChannel;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadManager {
    thread_size: usize,
    channel: Arc<AtomicChannel<Job>>,
    active_threads: Arc<AtomicUsize>,
    waiting_threads: Arc<AtomicUsize>,
    busy_threads: Arc<AtomicUsize>,
    workers: Vec<ThreadWorker>,
    dispatch_worker: AtomicUsize,
}

impl ThreadManager {
    pub fn new(thread_size: usize) -> Self {
        let channel: Arc<AtomicChannel<Job>> = Arc::new(AtomicChannel::new());
        let active_threads: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let waiting_threads: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let busy_threads: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let workers: Vec<ThreadWorker> = Self::create_workers(
            thread_size,
            channel.clone(),
            active_threads.clone(),
            waiting_threads.clone(),
            busy_threads.clone(),
        );

        let dispatch_worker: AtomicUsize = AtomicUsize::new(0);

        ThreadManager {
            thread_size,
            channel,
            active_threads,
            waiting_threads,
            busy_threads,
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
            worker.send_channel_release();
        }

        for worker in self.workers.iter() {
            worker.join();
        }
    }

    pub fn has_finished(&self) -> bool {
        let active_threads: usize = self.get_active_threads();
        let job_queue: usize = self.get_job_queue();

        if active_threads == 0 && job_queue == 0 {
            return true;
        }

        let waiting_threads: usize = self.get_waiting_threads();
        let busy_threads: usize = self.get_busy_threads();

        if job_queue == 0
            && busy_threads == 0
            && active_threads > 0
            && waiting_threads > 0
            && active_threads == waiting_threads
        {
            for worker in self.workers.iter().take(waiting_threads) {
                worker.send_channel_release();
            }
        }
        false
    }

    pub fn send_join_signals(&self) {
        for worker in self.workers.iter() {
            worker.send_join_signal();
        }
    }

    pub fn get_active_threads(&self) -> usize {
        let active_workers = self.active_threads.load(Ordering::Acquire);
        active_workers
    }

    pub fn get_busy_threads(&self) -> usize {
        let busy_workers = self.busy_threads.load(Ordering::Acquire);
        busy_workers
    }

    pub fn get_waiting_threads(&self) -> usize {
        let waiting_workers = self.waiting_threads.load(Ordering::Acquire);
        waiting_workers
    }

    pub fn get_jobs_distribution(&self) -> Vec<usize> {
        let mut received_jobs: Vec<usize> = Vec::new();
        for worker in self.workers.iter() {
            received_jobs.push(worker.get_jobs_received());
        }
        received_jobs
    }

    pub fn get_job_queue(&self) -> usize {
        let job_queue: usize = self.channel.get_pending_count();
        job_queue
    }

    pub fn get_jobs_received(&self) -> usize {
        let jobs_received: usize = self.channel.get_received_count();
        jobs_received
    }

    pub fn get_jobs_completed(&self) -> usize {
        let mut jobs_completed = 0;
        for worker in self.workers.iter() {
            jobs_completed += worker.get_jobs_completed();
        }
        jobs_completed
    }

    pub fn set_thread_size(&mut self, thread_size: usize) {
        if thread_size > self.workers.len() {
            let additional_threads: usize = thread_size - self.workers.len();
            let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
            let active_threads: Arc<AtomicUsize> = self.active_threads.clone();
            let waiting_threads: Arc<AtomicUsize> = self.waiting_threads.clone();
            let busy_threads: Arc<AtomicUsize> = self.busy_threads.clone();
            let workers: Vec<ThreadWorker> = Self::create_workers(
                additional_threads,
                channel,
                active_threads,
                waiting_threads,
                busy_threads,
            );
            self.workers.extend(workers);
        } else if thread_size < self.workers.len() {
            let split_workers: Vec<ThreadWorker> = self.workers.split_off(thread_size);
            for worker in split_workers.iter() {
                worker.send_termination_signal();
            }
        }
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
    fn create_workers(
        thread_size: usize,
        channel: Arc<AtomicChannel<Job>>,
        active_threads: Arc<AtomicUsize>,
        waiting_threads: Arc<AtomicUsize>,
        busy_threads: Arc<AtomicUsize>,
    ) -> Vec<ThreadWorker> {
        let mut workers: Vec<ThreadWorker> = Vec::with_capacity(thread_size);

        for id in 0..thread_size {
            let worker: ThreadWorker = ThreadWorker::new(
                id,
                channel.clone(),
                active_threads.clone(),
                waiting_threads.clone(),
                busy_threads.clone(),
            );
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
    is_waiting: Arc<AtomicBool>,
    is_busy: Arc<AtomicBool>,
    active_threads: Arc<AtomicUsize>,
    waiting_threads: Arc<AtomicUsize>,
    busy_threads: Arc<AtomicUsize>,
    jobs_received: Arc<AtomicUsize>,
    jobs_completed: Arc<AtomicUsize>,
    join_signal: Arc<AtomicBool>,
    termination_signal: Arc<AtomicBool>,
}

impl ThreadWorker {
    fn new(
        id: usize,
        channel: Arc<AtomicChannel<Job>>,
        active_threads: Arc<AtomicUsize>,
        waiting_threads: Arc<AtomicUsize>,
        busy_threads: Arc<AtomicUsize>,
    ) -> Self {
        let thread: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
        let is_active: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let is_waiting: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let is_busy: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let jobs_received: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let jobs_completed: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let join_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let termination_signal: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        ThreadWorker {
            id,
            thread,
            channel,
            is_active,
            is_waiting,
            is_busy,
            active_threads,
            waiting_threads,
            busy_threads,
            jobs_received,
            jobs_completed,
            join_signal,
            termination_signal,
        }
    }

    fn id(&self) -> usize {
        self.id
    }

    fn start(&self) {
        if !self.is_active() {
            if let Ok(mut thread_guard) = self.thread.lock() {
                if let Some(existing_thread) = thread_guard.take() {
                    if !existing_thread.is_finished() {
                        self.send_termination_signal();
                    }
                    let _ = existing_thread.join();
                    if self.channel.get_pending_count() == 0 {
                        return;
                    }
                }
                let worker_loop = self.create_worker_loop();
                let thread: thread::JoinHandle<()> = thread::spawn(worker_loop);
                *thread_guard = Some(thread);
            }
        }
    }

    fn send<F>(&self, function: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(function);
        self.channel
            .send(job)
            .expect(&format!("Failed to send job to Worker [{}]", self.id));
        self.start();
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

    fn is_waiting(&self) -> bool {
        let is_waiting: bool = self.is_waiting.load(Ordering::Acquire);
        is_waiting
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

    fn get_jobs_received(&self) -> usize {
        let received_jobs: usize = self.jobs_received.load(Ordering::Acquire);
        received_jobs
    }

    fn get_jobs_completed(&self) -> usize {
        let timeouts: usize = self.jobs_completed.load(Ordering::Acquire);
        timeouts
    }

    fn send_join_signal(&self) {
        self.join_signal.store(true, Ordering::Release);
    }

    fn send_termination_signal(&self) {
        self.termination_signal.store(true, Ordering::Release);
        self.send_channel_release();
    }

    fn send_channel_release(&self) {
        let closure: Job = Box::new(move || {});
        self.channel
            .send_uncounted(Box::new(closure))
            .expect(&format!("Failed to release Worker [{}]", self.id));
    }

    fn set_worker_active(is_active: &Arc<AtomicBool>, active_threads: &Arc<AtomicUsize>) {
        is_active.store(true, Ordering::Release);
        active_threads.fetch_add(1, Ordering::Release);
    }

    fn unset_worker_active(is_active: &Arc<AtomicBool>, active_threads: &Arc<AtomicUsize>) {
        is_active.store(false, Ordering::Release);
        active_threads.fetch_sub(1, Ordering::Release);
    }

    fn set_worker_waiting(is_waiting: &Arc<AtomicBool>, waiting_threads: &Arc<AtomicUsize>) {
        is_waiting.store(true, Ordering::Release);
        waiting_threads.fetch_add(1, Ordering::Release);
    }

    fn unset_worker_waiting(is_waiting: &Arc<AtomicBool>, waiting_threads: &Arc<AtomicUsize>) {
        is_waiting.store(false, Ordering::Release);
        waiting_threads.fetch_sub(1, Ordering::Release);
    }

    fn set_worker_busy(is_busy: &Arc<AtomicBool>, busy_threads: &Arc<AtomicUsize>) {
        is_busy.store(true, Ordering::Release);
        busy_threads.fetch_add(1, Ordering::Release);
    }

    fn unset_worker_busy(is_busy: &Arc<AtomicBool>, busy_threads: &Arc<AtomicUsize>) {
        is_busy.store(false, Ordering::Release);
        busy_threads.fetch_sub(1, Ordering::Release);
    }

    fn create_worker_loop(&self) -> impl Fn() {
        let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
        let is_active: Arc<AtomicBool> = self.is_active.clone();
        let is_waiting: Arc<AtomicBool> = self.is_waiting.clone();
        let is_busy: Arc<AtomicBool> = self.is_busy.clone();
        let active_threads: Arc<AtomicUsize> = self.active_threads.clone();
        let waiting_threads: Arc<AtomicUsize> = self.waiting_threads.clone();
        let busy_threads: Arc<AtomicUsize> = self.busy_threads.clone();
        let jobs_received: Arc<AtomicUsize> = self.jobs_received.clone();
        let jobs_completed: Arc<AtomicUsize> = self.jobs_completed.clone();
        let join_signal: Arc<AtomicBool> = self.join_signal.clone();
        let termination_signal: Arc<AtomicBool> = self.termination_signal.clone();

        let worker_loop = move || {
            Self::set_worker_active(&is_active, &active_threads);
            while !termination_signal.load(Ordering::Acquire) {
                if join_signal.load(Ordering::Acquire) {
                    if channel.get_pending_count() == 0 {
                        break;
                    }
                }

                Self::set_worker_waiting(&is_waiting, &waiting_threads);
                let recv = channel.recv();
                Self::unset_worker_waiting(&is_waiting, &waiting_threads);
                if let Ok(job) = recv {
                    jobs_received.fetch_add(1, Ordering::Release);
                    Self::set_worker_busy(&is_busy, &busy_threads);

                    job();
                    Self::unset_worker_busy(&is_busy, &busy_threads);
                    jobs_completed.fetch_add(1, Ordering::Release);
                }
            }

            Self::unset_worker_active(&is_active, &active_threads);
            termination_signal.store(false, Ordering::Release);
        };
        worker_loop
    }
}
