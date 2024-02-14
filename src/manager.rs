use std::sync::Arc;

use crate::assert::assert_wpc;
use crate::channel::JobChannel;
use crate::channel::ResultChannel;
use crate::dispatch::DispatchCycle;
use crate::iterator::ResultIter;
use crate::iterator::YieldResultIter;
use crate::status::ManagerStatus;
use crate::worker::ThreadWorker;

type FnType<T> = Box<dyn Fn() -> T + Send + 'static>;

/// A thread manager for executing jobs in parallel.
/// This struct manages a pool of worker threads and distributes jobs among them.
///
/// # Type Parameters
/// - `F`: The type of the function or closure that the threads will execute.
/// - `T`: The type of the value returned by the function or closure.
///
/// # Fields
/// - `wpc`: The number of Workers-Per-Channel.
/// - `dispatch`: An instance of `DispatchCycle` to manage job distribution.
/// - `workers`: A vector of `ThreadWorker` instances representing the worker threads.
/// - `channels`: A vector of job channels for dispatching jobs to workers.
/// - `result_channel`: A channel for collecting the results of the jobs.
/// - `manager_status`: An instance of `ManagerStatus` to track the status of the manager.
pub struct ThreadManagerCore<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    wpc: usize,
    dispatch: DispatchCycle,
    workers: Vec<ThreadWorker<F, T>>,
    channels: Vec<Arc<JobChannel<F>>>,
    result_channel: Arc<ResultChannel<T>>,
    manager_status: Arc<ManagerStatus>,
}

impl<F, T> ThreadManagerCore<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    /// Creates a new instance of `ThreadManagerCore` with a specified number of worker threads.
    ///
    /// # Arguments
    /// - `size`: The number of worker threads to create.
    ///
    /// # Returns
    /// A new instance of `ThreadManagerCore`.
    pub fn new(size: usize) -> Self {
        let dispatch: DispatchCycle = DispatchCycle::new(size);
        let workers: Vec<ThreadWorker<F, T>> = Vec::with_capacity(size);
        let channels: Vec<Arc<JobChannel<F>>> = Vec::with_capacity(size);
        let result_channel: Arc<ResultChannel<T>> = Arc::new(ResultChannel::new());
        let manager_status: Arc<ManagerStatus> = Arc::new(ManagerStatus::new());

        let mut manager: ThreadManagerCore<F, T> = Self {
            wpc: 1,
            dispatch,
            workers,
            channels,
            result_channel,
            manager_status,
        };
        manager.create_workers(size);
        manager
    }

    /// Creates a new instance of `ThreadManagerCore` with a specified number of worker threads
    /// and a specific workers-per-channel ratio.
    ///
    /// # Arguments
    /// - `size`: The number of worker threads to create.
    /// - `wpc`: The number of workers per channel.
    ///
    /// # Returns
    /// A new instance of `ThreadManagerCore` with the specified configuration.
    pub fn new_asymmetric(size: usize, wpc: usize) -> Self {
        assert_wpc(size, wpc);
        let dispatch: DispatchCycle = DispatchCycle::new(size);
        let workers: Vec<ThreadWorker<F, T>> = Vec::with_capacity(size);
        let channels: Vec<Arc<JobChannel<F>>> = Vec::with_capacity(size);
        let result_channel: Arc<ResultChannel<T>> = Arc::new(ResultChannel::new());
        let manager_status: Arc<ManagerStatus> = Arc::new(ManagerStatus::new());

        let mut manager: ThreadManagerCore<F, T> = Self {
            wpc,
            dispatch,
            workers,
            channels,
            result_channel,
            manager_status,
        };
        manager.create_workers(size);
        manager
    }

    /// Executes a given function by sending it to an available worker thread.
    ///
    /// # Arguments
    /// - `function`: The function to be executed by the worker thread.
    pub fn execute(&self, function: F) {
        let id: usize = self.dispatch.fetch_and_update();
        let worker: &ThreadWorker<F, T> = &self.workers[id];
        worker.send(function);
    }

    /// Resizes the pool of worker threads.
    ///
    /// # Arguments
    /// - `size`: The new size of the worker pool.
    pub fn resize(&mut self, size: usize) {
        assert_wpc(size, self.wpc);
        let dispatch_size: usize = self.dispatch.fetch_size();

        if size > self.workers.len() {
            let additional_size: usize = size - self.workers.len();
            self.start_workers(dispatch_size, self.workers.len());
            self.create_workers(additional_size);
            self.dispatch.set_size(size);
        } else if size < dispatch_size {
            self.send_release_workers(size, dispatch_size);
            self.dispatch.set_size(size);
        } else if size > dispatch_size {
            self.start_workers(dispatch_size, size);
            self.dispatch.set_size(size);
        }
    }
}

impl<F, T> ThreadManagerCore<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    fn get_channel(&self, id: usize) -> Arc<JobChannel<F>> {
        let channel_id: usize = id / self.wpc;
        self.channels[channel_id].clone()
    }

    fn create_channels(&mut self, size: usize) {
        for _ in 0..(size / self.wpc) {
            let channel: JobChannel<F> = JobChannel::new();
            let channel: Arc<JobChannel<F>> = Arc::new(channel);
            self.channels.push(channel);
        }
    }

    fn create_workers(&mut self, size: usize) {
        self.create_channels(size);
        let worker_size: usize = self.workers.len();

        for idx in 0..size {
            let id: usize = idx + worker_size;
            let job_channel: Arc<JobChannel<F>> = self.get_channel(id);
            let result_channel: Arc<ResultChannel<T>> = self.result_channel.clone();
            let manager_status: Arc<ManagerStatus> = self.manager_status.clone();
            let worker: ThreadWorker<F, T> =
                ThreadWorker::new(id, job_channel, result_channel, manager_status);

            worker.start();
            self.workers.push(worker);
        }
    }
}

impl<F, T> ThreadManagerCore<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    pub fn join(&self) {
        self.send_release_workers(0, self.workers.len());
        self.join_workers(0, self.workers.len());
        self.clear_channels(0, self.channels.len());
    }

    pub fn terminate_all(&self) {
        self.set_termination_workers(0, self.workers.len());
        self.send_release_workers(0, self.workers.len());
        self.join_workers(0, self.workers.len());
        self.clear_channels(0, self.channels.len());
    }

    pub fn job_distribution(&self) -> Vec<usize> {
        let mut distribution: Vec<usize> = Vec::with_capacity(self.workers.len());
        for worker in self.workers.iter() {
            distribution.push(worker.status().received());
        }
        distribution
    }

    pub fn has_finished(&self) -> bool {
        for job_channel in self.channels.iter() {
            if !job_channel.is_finished() {
                return false;
            }
        }
        true
    }

    pub fn results<'a>(&'a self) -> ResultIter<'a, T> {
        ResultIter::new(&self.result_channel)
    }

    pub fn yield_results<'a>(&'a self) -> YieldResultIter<'a, F, T> {
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
        for job_channel in self.channels.iter() {
            queue += job_channel.status().pending();
        }
        queue
    }

    pub fn sent_jobs(&self) -> usize {
        let mut sent: usize = 0;
        for job_channel in self.channels.iter() {
            sent += job_channel.status().sent();
        }
        sent
    }

    pub fn received_jobs(&self) -> usize {
        let mut received: usize = 0;
        for job_channel in self.channels.iter() {
            received += job_channel.status().received();
        }
        received
    }

    pub fn concluded_jobs(&self) -> usize {
        let mut concluded: usize = 0;
        for job_channel in self.channels.iter() {
            concluded += job_channel.status().concluded();
        }
        concluded
    }
}

impl<F, T> ThreadManagerCore<F, T>
where
    F: Fn() -> T + Send + 'static,
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

    fn clear_channels(&self, st: usize, en: usize) {
        for job_channel in self.channels[st..en].iter() {
            job_channel.clear();
        }
    }

    fn set_termination_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.set_termination(true);
        }
    }

    fn send_release_workers(&self, st: usize, en: usize) {
        for worker in self.workers[st..en].iter() {
            worker.send_release();
        }
    }
}

impl<F, T> Drop for ThreadManagerCore<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    fn drop(&mut self) {
        self.terminate_all();
    }
}

/// A dynamic dispatch version of `ThreadManagerCore` for managing threads that execute functions
/// returning a specific type `T`.
///
/// # Type Parameters
/// - `T`: The type of the value returned by the functions executed by the threads.
pub struct ThreadManager<T>
where
    T: Send + 'static,
{
    manager: ThreadManagerCore<FnType<T>, T>,
}

impl<T> ThreadManager<T>
where
    T: Send + 'static,
{
    /// Creates a new instance of `ThreadManager` with a specified number of worker threads.
    ///
    /// # Arguments
    /// - `size`: The number of worker threads to create.
    ///
    /// # Returns
    /// A new instance of `ThreadManager`.
    pub fn new(size: usize) -> Self {
        let manager: ThreadManagerCore<FnType<T>, T> = ThreadManagerCore::new(size);
        Self { manager }
    }

    /// Creates a new instance of `ThreadManager` with a specified number of worker threads
    /// and a specific workers-per-channel ratio.
    ///
    /// # Arguments
    /// - `size`: The number of worker threads to create.
    /// - `wpc`: The number of workers per channel.
    ///
    /// # Returns
    /// A new instance of `ThreadManager` with the specified configuration.
    pub fn new_asymmetric(size: usize, wpc: usize) -> Self {
        let manager: ThreadManagerCore<FnType<T>, T> = ThreadManagerCore::new_asymmetric(size, wpc);
        Self { manager }
    }

    /// Executes a given function by sending it to an available worker thread.
    ///
    /// # Type Parameters
    /// - `F`: The type of the function to execute.
    ///
    /// # Arguments
    /// - `function`: The function to be executed by the worker thread.
    pub fn execute<F>(&self, function: F)
    where
        F: Fn() -> T + Send + 'static,
    {
        self.manager.execute(Box::new(function))
    }

    /// Resizes the pool of worker threads.
    ///
    /// # Arguments
    /// - `size`: The new size of the worker pool.
    pub fn resize(&mut self, size: usize) {
        self.manager.resize(size)
    }

    pub fn join(&self) {
        self.manager.join();
    }

    pub fn terminate_all(&self) {
        self.manager.terminate_all()
    }

    pub fn job_distribution(&self) -> Vec<usize> {
        self.manager.job_distribution()
    }

    pub fn has_finished(&self) -> bool {
        self.manager.has_finished()
    }

    pub fn results<'a>(&'a self) -> ResultIter<'a, T> {
        self.manager.results()
    }

    pub fn yield_results<'a>(&'a self) -> YieldResultIter<'a, FnType<T>, T> {
        self.manager.yield_results()
    }

    pub fn active_threads(&self) -> usize {
        self.manager.active_threads()
    }

    pub fn busy_threads(&self) -> usize {
        self.manager.busy_threads()
    }

    pub fn waiting_threads(&self) -> usize {
        self.manager.waiting_threads()
    }

    pub fn job_queue(&self) -> usize {
        self.manager.job_queue()
    }

    pub fn sent_jobs(&self) -> usize {
        self.manager.sent_jobs()
    }

    pub fn received_jobs(&self) -> usize {
        self.manager.received_jobs()
    }

    pub fn concluded_jobs(&self) -> usize {
        self.manager.concluded_jobs()
    }
}
