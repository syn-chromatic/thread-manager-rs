use std::sync::Arc;

use crate::channel::JobChannel;
use crate::channel::ResultChannel;
use crate::dispatch::DispatchCycle;
use crate::iterator::ResultIter;
use crate::iterator::YieldResultIter;
use crate::status::ManagerStatus;
use crate::types::FnType;
use crate::worker::ThreadWorker;

pub struct ThreadManager<T>
where
    T: Send + 'static,
{
    dispatch: DispatchCycle,
    workers: Vec<ThreadWorker<FnType<T>, T>>,
    result_channel: Arc<ResultChannel<T>>,
    manager_status: Arc<ManagerStatus>,
}

impl<T> ThreadManager<T>
where
    T: Send + 'static,
{
    pub fn new(size: usize) -> Self {
        let dispatch: DispatchCycle = DispatchCycle::new(size);
        let workers: Vec<ThreadWorker<FnType<T>, T>> = Vec::with_capacity(size);
        let result_channel: Arc<ResultChannel<T>> = Arc::new(ResultChannel::new());
        let manager_status: Arc<ManagerStatus> = Arc::new(ManagerStatus::new());

        let mut manager: ThreadManager<T> = Self {
            dispatch,
            workers,
            result_channel,
            manager_status,
        };
        manager.create_workers(size);
        manager
    }

    pub fn execute<F>(&self, function: F)
    where
        F: Fn() -> T + Send + 'static,
    {
        let id: usize = self.dispatch.fetch_and_update();
        let worker: &ThreadWorker<FnType<T>, T> = &self.workers[id];
        worker.send(Box::new(function));
    }

    pub fn resize(&mut self, size: usize) {
        let dispatch_size: usize = self.dispatch.fetch_size();

        if size > self.workers.len() {
            let additional_size: usize = size - self.workers.len();
            self.start_workers(dispatch_size, self.workers.len());
            self.create_workers(additional_size);
            self.dispatch.set_size(size);
        } else if size < dispatch_size {
            self.set_termination_workers(size, dispatch_size);
            self.dispatch.set_size(size);
        } else if size > dispatch_size {
            self.start_workers(dispatch_size, size);
            self.dispatch.set_size(size);
        }
    }
}

impl<T> ThreadManager<T>
where
    T: Send + 'static,
{
    fn create_workers(&mut self, size: usize) {
        let worker_size: usize = self.workers.len();

        for idx in 0..size {
            let id: usize = idx + worker_size;
            let result_channel: Arc<ResultChannel<T>> = self.result_channel.clone();
            let manager_status: Arc<ManagerStatus> = self.manager_status.clone();
            let worker: ThreadWorker<FnType<T>, T> =
                ThreadWorker::new(id, result_channel, manager_status);

            worker.start();
            self.workers.push(worker);
        }
    }
}

impl<T> ThreadManager<T>
where
    T: Send + 'static,
{
    pub fn join(&self) {
        self.send_release_workers(0, self.workers.len());
        self.join_workers(0, self.workers.len());
        self.clear_job_channel_workers(0, self.workers.len());
    }

    pub fn terminate_all(&self) {
        self.set_termination_workers(0, self.workers.len());
        self.send_release_workers(0, self.workers.len());
        self.join_workers(0, self.workers.len());
        self.clear_job_channel_workers(0, self.workers.len());
    }

    pub fn job_distribution(&self) -> Vec<usize> {
        let mut distribution: Vec<usize> = Vec::with_capacity(self.workers.len());
        for worker in self.workers.iter() {
            let job_channel: &Arc<JobChannel<FnType<T>>> = worker.job_channel();
            distribution.push(job_channel.status().received());
        }
        distribution
    }

    pub fn has_finished(&self) -> bool {
        for worker in self.workers.iter() {
            if !worker.job_channel().is_finished() {
                return false;
            }
        }
        true
    }

    pub fn results<'a>(&'a self) -> ResultIter<'a, T> {
        ResultIter::new(&self.result_channel)
    }

    pub fn yield_results<'a>(&'a self) -> YieldResultIter<'a, T> {
        YieldResultIter::new(&self.workers, &self.result_channel)
    }

    pub fn active_threads(&self) -> usize {
        self.manager_status.active_threads()
    }

    pub fn busy_threads(&self) -> usize {
        self.manager_status.busy_threads()
    }

    pub fn waiting_threads(&self) -> usize {
        self.manager_status.waiting_threads()
    }

    pub fn job_queue(&self) -> usize {
        let mut queue: usize = 0;
        for worker in self.workers.iter() {
            let job_channel: &Arc<JobChannel<FnType<T>>> = worker.job_channel();
            queue += job_channel.status().pending();
        }
        queue
    }

    pub fn sent_jobs(&self) -> usize {
        let mut sent: usize = 0;
        for worker in self.workers.iter() {
            let job_channel: &Arc<JobChannel<FnType<T>>> = worker.job_channel();
            sent += job_channel.status().sent();
        }
        sent
    }

    pub fn received_jobs(&self) -> usize {
        let mut received: usize = 0;
        for worker in self.workers.iter() {
            let job_channel: &Arc<JobChannel<FnType<T>>> = worker.job_channel();
            received += job_channel.status().received();
        }
        received
    }

    pub fn concluded_jobs(&self) -> usize {
        let mut concluded: usize = 0;
        for worker in self.workers.iter() {
            let job_channel: &Arc<JobChannel<FnType<T>>> = worker.job_channel();
            concluded += job_channel.status().concluded();
        }
        concluded
    }
}

impl<T> ThreadManager<T>
where
    T: Send + 'static,
{
    fn start_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.start();
        }
    }

    fn join_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.join();
        }
    }

    fn clear_job_channel_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.job_channel().clear();
        }
    }

    fn set_termination_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.set_termination();
        }
    }

    fn send_release_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.send_release();
        }
    }
}

impl<T> Drop for ThreadManager<T>
where
    T: Send + 'static,
{
    fn drop(&mut self) {
        self.terminate_all();
    }
}
