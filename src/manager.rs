use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::channel::AtomicChannel;
use crate::status::ManagerStatus;
use crate::worker::ThreadWorker;

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct RoundRobinDispatch {
    max: usize,
    value: AtomicUsize,
}

impl RoundRobinDispatch {
    pub fn new(max: usize) -> Self {
        let value: AtomicUsize = AtomicUsize::new(0);
        Self { max, value }
    }

    pub fn fetch_and_update(&self) -> usize {
        let dispatch: usize = self.value.load(Ordering::Acquire);
        if dispatch >= (self.max - 1) {
            self.value.store(0, Ordering::Release);
        } else {
            self.value.store(dispatch + 1, Ordering::Release);
        }
        dispatch
    }
}

pub struct ThreadManager {
    size: usize,
    channel: Arc<AtomicChannel<Job>>,
    status: Arc<ManagerStatus>,
    workers: Vec<ThreadWorker>,
    dispatch: RoundRobinDispatch,
}

impl ThreadManager {
    pub fn new(size: usize) -> Self {
        let channel: Arc<AtomicChannel<Job>> = Arc::new(AtomicChannel::new());
        let status: Arc<ManagerStatus> = Arc::new(ManagerStatus::new());
        let dispatch: RoundRobinDispatch = RoundRobinDispatch::new(size);

        let workers: Vec<ThreadWorker> =
            Self::create_workers(size, channel.clone(), status.clone());

        ThreadManager {
            size,
            channel,
            status,
            workers,
            dispatch,
        }
    }

    pub fn execute<F>(&self, function: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let dispatch: usize = self.dispatch.fetch_and_update();
        let worker: &ThreadWorker = &self.workers[dispatch];
        worker.send(function);
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

    pub fn set_thread_size(&mut self, size: usize) {
        if size > self.workers.len() {
            let additional_size: usize = size - self.workers.len();
            let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
            let status: Arc<ManagerStatus> = self.status.clone();
            let workers: Vec<ThreadWorker> = Self::create_workers(additional_size, channel, status);
            self.workers.extend(workers);
            self.size += additional_size;
        } else if size < self.workers.len() {
            let split_workers: Vec<ThreadWorker> = self.workers.split_off(size);
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

    pub fn clear_job_queue(&self) {
        self.channel.clear_receiver();
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

    pub fn has_finished(&self) -> bool {
        let sent_jobs: usize = self.get_sent_jobs();
        let completed_jobs: usize = self.get_completed_jobs();

        if completed_jobs != sent_jobs {
            return false;
        }
        true
    }

    pub fn get_active_threads(&self) -> usize {
        self.status.get_active_threads()
    }

    pub fn get_busy_threads(&self) -> usize {
        self.status.get_busy_threads()
    }

    pub fn get_waiting_threads(&self) -> usize {
        self.status.get_waiting_threads()
    }

    pub fn get_job_distribution(&self) -> Vec<usize> {
        let mut received_jobs: Vec<usize> = Vec::new();
        for worker in self.workers.iter() {
            received_jobs.push(worker.get_received_jobs());
        }
        received_jobs
    }

    pub fn get_job_queue(&self) -> usize {
        let job_queue: usize = self.channel.get_pending_count();
        job_queue
    }

    pub fn get_received_jobs(&self) -> usize {
        let received_jobs: usize = self.channel.get_received_count();
        received_jobs
    }

    pub fn get_sent_jobs(&self) -> usize {
        let sent_jobs: usize = self.channel.get_sent_count();
        sent_jobs
    }

    pub fn get_completed_jobs(&self) -> usize {
        let overall_completed_jobs: usize = self.channel.get_concluded_count();
        overall_completed_jobs
    }
}

impl ThreadManager {
    fn create_workers(
        size: usize,
        channel: Arc<AtomicChannel<Job>>,
        status: Arc<ManagerStatus>,
    ) -> Vec<ThreadWorker> {
        let mut workers: Vec<ThreadWorker> = Vec::with_capacity(size);

        for id in 0..size {
            let worker: ThreadWorker = ThreadWorker::new(id, channel.clone(), status.clone());
            worker.start();
            workers.push(worker);
        }
        workers
    }
}
