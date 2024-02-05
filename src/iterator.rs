use std::sync::Arc;

use crossbeam_channel::RecvError;

use crate::channel::ResultChannel;
use crate::worker::ThreadWorker;

pub struct ResultIter<'a, T>
where
    T: Send + 'static,
{
    result_channel: &'a Arc<ResultChannel<T>>,
}

impl<'a, T> ResultIter<'a, T>
where
    T: Send + 'static,
{
    pub fn new(result_channel: &'a Arc<ResultChannel<T>>) -> Self {
        Self { result_channel }
    }

    pub fn has_results(&self) -> bool {
        !self.result_channel.is_finished()
    }
}

impl<'a, T> Iterator for ResultIter<'a, T>
where
    T: Send + 'static,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_results() {
            let result: Result<T, RecvError> = self.result_channel.recv();
            self.result_channel.status().set_concluded(true);
            if let Ok(result) = result {
                return Some(result);
            }
        }
        None
    }
}

pub struct YieldResultIter<'a, F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    workers: &'a Vec<ThreadWorker<F, T>>,
    result_channel: &'a Arc<ResultChannel<T>>,
}

impl<'a, F, T> YieldResultIter<'a, F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    pub fn new(
        workers: &'a Vec<ThreadWorker<F, T>>,
        result_channel: &'a Arc<ResultChannel<T>>,
    ) -> Self {
        Self {
            workers,
            result_channel,
        }
    }

    pub fn has_jobs(&self) -> bool {
        for worker in self.workers.iter() {
            if !worker.job_channel().is_finished() {
                return true;
            }
        }
        false
    }

    pub fn has_results(&self) -> bool {
        !self.result_channel.is_finished()
    }
}

impl<'a, F, T> Iterator for YieldResultIter<'a, F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_jobs() || self.has_results() {
            let result: Result<T, RecvError> = self.result_channel.recv();
            self.result_channel.status().set_concluded(true);
            if let Ok(result) = result {
                return Some(result);
            }
        }
        None
    }
}
