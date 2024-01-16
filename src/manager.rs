use std::sync::Arc;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::channel::AtomicChannel;
use crate::status::ManagerStatus;
use crate::types::Job;
use crate::worker::ThreadWorker;

pub struct ThreadManager {
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
        let workers: Vec<ThreadWorker> = Vec::with_capacity(size);

        let mut manager: ThreadManager = ThreadManager {
            channel,
            status,
            workers,
            dispatch,
        };
        manager.create_workers(size);
        manager
    }

    pub fn execute<F>(&self, function: F)
    where
        F: Fn() + Send + 'static,
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
            worker.send_release_signal();
        }

        for worker in self.workers.iter() {
            worker.join();
        }
    }

    pub fn terminate_all(&self) {
        for worker in self.workers.iter() {
            worker.send_termination_signal();
        }

        for worker in self.workers.iter() {
            worker.join();
        }

        self.channel.clear();
    }

    pub fn set_thread_size(&mut self, size: usize) {
        if size > self.workers.len() {
            let additional_size: usize = size - self.workers.len();
            self.create_workers(additional_size);
        } else if size < self.workers.len() {
            let split_workers: Vec<ThreadWorker> = self.workers.split_off(size);
            for worker in split_workers.iter() {
                worker.send_termination_signal();
            }
        }
    }

    pub fn has_finished(&self) -> bool {
        let sent_jobs: usize = self.sent_jobs();
        let completed_jobs: usize = self.completed_jobs();

        if completed_jobs != sent_jobs {
            return false;
        }
        true
    }

    pub fn active_threads(&self) -> usize {
        self.status.active_threads()
    }

    pub fn busy_threads(&self) -> usize {
        self.status.busy_threads()
    }

    pub fn waiting_threads(&self) -> usize {
        self.status.waiting_threads()
    }

    pub fn job_queue(&self) -> usize {
        self.channel.status().pending()
    }

    pub fn sent_jobs(&self) -> usize {
        self.channel.status().sent()
    }

    pub fn received_jobs(&self) -> usize {
        self.channel.status().received()
    }

    pub fn completed_jobs(&self) -> usize {
        self.channel.status().concluded()
    }

    pub fn job_distribution(&self) -> Vec<usize> {
        let mut received_jobs: Vec<usize> = Vec::new();
        for worker in self.workers.iter() {
            received_jobs.push(worker.status().received());
        }
        received_jobs
    }
}

impl ThreadManager {
    fn create_workers(&mut self, size: usize) {
        let worker_size: usize = self.workers.len();

        for idx in 0..size {
            let id: usize = idx + worker_size;
            let channel: Arc<AtomicChannel<Box<dyn Fn() + Send>>> = self.channel.clone();
            let manager_status: Arc<ManagerStatus> = self.status.clone();
            let worker: ThreadWorker = ThreadWorker::new(id, channel, manager_status);

            worker.start();
            self.workers.push(worker);
        }
    }
}

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
