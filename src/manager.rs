use std::sync::mpsc::RecvError;
use std::sync::Arc;

use crate::channel::JobChannel;
use crate::channel::ResultChannel;
use crate::dispatch::DispatchID;
use crate::status::ManagerStatus;
use crate::worker::ThreadWorker;

pub type FnType<T> = Box<dyn Fn() -> T + Send + 'static>;

pub struct ThreadManager<T>
where
    T: Send + 'static,
{
    dispatch: DispatchID,
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
        let dispatch: DispatchID = DispatchID::new(size);
        let job_channel: Arc<JobChannel<FnType<T>>> = Arc::new(JobChannel::new());
        let result_channel: Arc<ResultChannel<T>> = Arc::new(ResultChannel::new());
        let manager_status: Arc<ManagerStatus> = Arc::new(ManagerStatus::new());
        let workers: Vec<ThreadWorker<FnType<T>, T>> = Vec::with_capacity(size);

        let mut manager: ThreadManager<T> = ThreadManager {
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
        if size > self.workers.len() {
            let additional_size: usize = size - self.workers.len();
            self.create_workers(additional_size);
        } else if size < self.workers.len() {
            for idx in size..self.workers.len() {
                (&self.workers[idx]).send_termination_signal();
            }

            for worker in self.workers.iter() {
                worker.send_release_signal();
            }

            let _ = self.workers.split_off(size);
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
            worker.send_release_signal();
        }

        for worker in self.workers.iter() {
            worker.join();
        }

        self.job_channel.clear();
    }

    pub fn job_distribution(&self) -> Vec<usize> {
        let mut received_jobs: Vec<usize> = Vec::new();
        for worker in self.workers.iter() {
            received_jobs.push(worker.status().received());
        }
        received_jobs
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

impl<'a, T> ResultIterator<'a, T> {
    fn jobs_finished(&self) -> bool {
        let sent: usize = self.job_channel.status().sent();
        let concluded: usize = self.job_channel.status().concluded();

        if concluded != sent {
            return false;
        }
        true
    }

    fn results_finished(&self) -> bool {
        let sent: usize = self.result_channel.status().sent();
        let concluded: usize = self.result_channel.status().concluded();

        if concluded != sent {
            return false;
        }
        true
    }
}

impl<'a, T> Iterator for ResultIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.jobs_finished() || !self.results_finished() {
            let result: Result<T, RecvError> = self.result_channel.recv();
            self.result_channel.status().add_concluded();
            if let Ok(result) = result {
                return Some(result);
            }
        }
        None
    }
}
