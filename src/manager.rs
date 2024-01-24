use std::sync::mpsc::RecvError;
use std::sync::Arc;

use crate::channel::JobChannel;
use crate::channel::ResultChannel;
use crate::dispatch::DispatchCycle;
use crate::status::ManagerStatus;
use crate::types::FnType;
use crate::worker::ThreadWorker;

pub struct ThreadManager<T>
where
    T: Send + 'static,
{
    dispatch: DispatchCycle,
    workers: Vec<ThreadWorker<FnType<T>, T>>,
    job_channel: Arc<JobChannel<FnType<T>>>,
    result_channel: Arc<ResultChannel<T>>,
    manager_status: Arc<ManagerStatus>,
}

impl<T> ThreadManager<T>
where
    T: Send + 'static,
{
    pub fn new(size: usize) -> Self {
        let dispatch: DispatchCycle = DispatchCycle::new(size);
        let job_channel: Arc<JobChannel<FnType<T>>> = Arc::new(JobChannel::new());
        let result_channel: Arc<ResultChannel<T>> = Arc::new(ResultChannel::new());
        let manager_status: Arc<ManagerStatus> = Arc::new(ManagerStatus::new());
        let workers: Vec<ThreadWorker<FnType<T>, T>> = Vec::with_capacity(size);

        let mut manager: ThreadManager<T> = Self {
            dispatch,
            workers,
            job_channel,
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

    pub fn set_thread_size(&mut self, size: usize) {
        let max_id: usize = self.dispatch.fetch_max();

        if size > self.workers.len() {
            let additional_size: usize = size - self.workers.len();
            self.start_workers(max_id, self.workers.len());
            self.create_workers(additional_size);
            self.dispatch.set_max(size);
        } else if size < max_id {
            self.send_termination_workers(size, max_id);
            self.dispatch.set_max(size);
        } else if size > max_id {
            self.start_workers(max_id, size);
            self.dispatch.set_max(size);
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
            let job_channel: Arc<JobChannel<FnType<T>>> = self.job_channel.clone();
            let result_channel: Arc<ResultChannel<T>> = self.result_channel.clone();
            let manager_status: Arc<ManagerStatus> = self.manager_status.clone();
            let worker: ThreadWorker<FnType<T>, T> =
                ThreadWorker::new(id, job_channel, result_channel, manager_status);

            worker.start();
            self.workers.push(worker);
        }
    }
}

impl<T> ThreadManager<T>
where
    T: Send + 'static,
{
    pub fn results<'a>(&'a self) -> ResultIterator<'a, T> {
        ResultIterator::new(&self.job_channel, &self.result_channel)
    }

    pub fn join(&self) {
        self.send_release_workers(0, self.workers.len());
        self.join_workers(0, self.workers.len());
        self.job_channel.clear();
    }

    pub fn terminate_all(&self) {
        self.send_termination_workers(0, self.workers.len());
        self.send_release_workers(0, self.workers.len());
        self.join_workers(0, self.workers.len());
        self.job_channel.clear();
    }

    pub fn job_distribution(&self) -> Vec<usize> {
        let mut received_jobs: Vec<usize> = Vec::with_capacity(self.workers.len());
        for worker in self.workers.iter() {
            received_jobs.push(worker.status().received());
        }
        received_jobs
    }

    pub fn has_finished(&self) -> bool {
        self.job_channel.is_finished()
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
        self.job_channel.status().pending()
    }

    pub fn sent_jobs(&self) -> usize {
        self.job_channel.status().sent()
    }

    pub fn received_jobs(&self) -> usize {
        self.job_channel.status().received()
    }

    pub fn completed_jobs(&self) -> usize {
        self.job_channel.status().concluded()
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

    fn send_termination_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.send_termination_signal();
        }
    }

    fn send_release_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.send_release_signal();
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

pub struct ResultIterator<'a, T> {
    job_channel: &'a Arc<JobChannel<FnType<T>>>,
    result_channel: &'a Arc<ResultChannel<T>>,
}

impl<'a, T> ResultIterator<'a, T> {
    pub fn new(
        job_channel: &'a Arc<JobChannel<FnType<T>>>,
        result_channel: &'a Arc<ResultChannel<T>>,
    ) -> Self {
        Self {
            job_channel,
            result_channel,
        }
    }
}

impl<'a, T> Iterator for ResultIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.job_channel.is_finished() || !self.result_channel.is_finished() {
            let result: Result<T, RecvError> = self.result_channel.recv();
            self.result_channel.status().add_concluded();
            if let Ok(result) = result {
                return Some(result);
            }
        }
        None
    }
}
