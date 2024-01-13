use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::channel::AtomicChannel;
use crate::status::ManagerStatus;
use crate::worker::ThreadWorker;

pub type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadManager {
    thread_size: usize,
    channel: Arc<AtomicChannel<Job>>,
    manager_status: Arc<ManagerStatus>,
    workers: Vec<ThreadWorker>,
    dispatch_worker: AtomicUsize,
}

impl ThreadManager {
    pub fn new(thread_size: usize) -> Self {
        let channel: Arc<AtomicChannel<Job>> = Arc::new(AtomicChannel::new());
        let manager_status: Arc<ManagerStatus> = Arc::new(ManagerStatus::new());
        let dispatch_worker: AtomicUsize = AtomicUsize::new(0);

        let workers: Vec<ThreadWorker> =
            Self::create_workers(thread_size, channel.clone(), manager_status.clone());

        ThreadManager {
            thread_size,
            channel,
            manager_status,
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

    pub fn set_thread_size(&mut self, thread_size: usize) {
        if thread_size > self.workers.len() {
            let additional_threads: usize = thread_size - self.workers.len();
            let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
            let manager_status: Arc<ManagerStatus> = self.manager_status.clone();
            let workers: Vec<ThreadWorker> =
                Self::create_workers(additional_threads, channel, manager_status);
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
        self.manager_status.get_active_threads()
    }

    pub fn get_busy_threads(&self) -> usize {
        self.manager_status.get_busy_threads()
    }

    pub fn get_waiting_threads(&self) -> usize {
        self.manager_status.get_waiting_threads()
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
        thread_size: usize,
        channel: Arc<AtomicChannel<Job>>,
        manager_status: Arc<ManagerStatus>,
    ) -> Vec<ThreadWorker> {
        let mut workers: Vec<ThreadWorker> = Vec::with_capacity(thread_size);

        for id in 0..thread_size {
            let worker: ThreadWorker =
                ThreadWorker::new(id, channel.clone(), manager_status.clone());
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
