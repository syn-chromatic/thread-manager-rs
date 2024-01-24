use std::cell::Cell;
use std::sync::mpsc::RecvError;
use std::sync::Arc;
use std::thread;

use crate::channel::JobChannel;
use crate::channel::MessageKind;
use crate::channel::ResultChannel;

use crate::status::ManagerStatus;
use crate::status::WorkerSignals;
use crate::status::WorkerStatus;

pub struct ThreadWorker<F, T>
where
    F: Fn() -> T + Send + 'static,
{
    id: usize,
    thread: Cell<Option<thread::JoinHandle<()>>>,
    signals: Arc<WorkerSignals>,
    job_channel: Arc<JobChannel<F>>,
    result_channel: Arc<ResultChannel<T>>,
    manager_status: Arc<ManagerStatus>,
    worker_status: Arc<WorkerStatus>,
}

impl<F, T> ThreadWorker<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    pub fn new(
        id: usize,
        job_channel: Arc<JobChannel<F>>,
        result_channel: Arc<ResultChannel<T>>,
        manager_status: Arc<ManagerStatus>,
    ) -> Self {
        let thread: Cell<Option<thread::JoinHandle<()>>> = Cell::new(None);
        let signals: Arc<WorkerSignals> = Arc::new(WorkerSignals::new());
        let worker_status: Arc<WorkerStatus> = Arc::new(WorkerStatus::new());

        Self {
            id,
            thread,
            signals,
            job_channel,
            result_channel,
            manager_status,
            worker_status,
        }
    }

    pub fn start(&self) {
        if !self.worker_status.is_active() {
            Self::set_active(&self.manager_status, &self.worker_status, true);
            let thread: thread::JoinHandle<()> = thread::spawn(self.create_worker());
            self.thread.set(Some(thread));
        }
    }

    pub fn send(&self, job: F) {
        self.job_channel
            .send(job)
            .expect(&format!("Failed to send job to Worker [{}]", self.id()));
        self.start();
    }
}

impl<F, T> ThreadWorker<F, T>
where
    F: Fn() -> T + Send + 'static,
{
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn status(&self) -> &Arc<WorkerStatus> {
        &self.worker_status
    }

    pub fn join(&self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    pub fn send_termination_signal(&self) {
        self.signals.set_termination_signal(true);
    }

    pub fn send_release_signal(&self) {
        self.job_channel
            .send_release()
            .expect(&format!("Failed to release Worker [{}]", self.id()));
    }
}

impl<F, T> ThreadWorker<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    fn set_active(
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
        state: bool,
    ) {
        worker_status.set_active(state);
        manager_status.set_active(state);
    }

    fn set_waiting(
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
        state: bool,
    ) {
        worker_status.set_waiting(state);
        manager_status.set_waiting(state);
    }

    fn set_busy(
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
        state: bool,
    ) {
        worker_status.set_busy(state);
        manager_status.set_busy(state);
    }

    fn start_worker(
        signals: &Arc<WorkerSignals>,
        job_channel: &Arc<JobChannel<F>>,
        result_channel: &Arc<ResultChannel<T>>,
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
    ) {
        while !signals.termination_signal() {
            Self::set_waiting(&manager_status, &worker_status, true);
            let recv: Result<MessageKind<F>, RecvError> = job_channel.recv();
            Self::set_waiting(&manager_status, &worker_status, false);
            if let Ok(message) = recv {
                match message {
                    MessageKind::Job(job) => {
                        worker_status.add_received();
                        Self::set_busy(&manager_status, &worker_status, true);
                        result_channel.send(job()).expect("Failed to send result");
                        Self::set_busy(&manager_status, &worker_status, false);
                        job_channel.status().add_concluded();
                    }
                    MessageKind::Release => {
                        break;
                    }
                }
            }
        }
    }

    fn create_worker(&self) -> impl Fn() {
        let signals: Arc<WorkerSignals> = self.signals.clone();
        let job_channel: Arc<JobChannel<F>> = self.job_channel.clone();
        let result_channel: Arc<ResultChannel<T>> = self.result_channel.clone();
        let manager_status: Arc<ManagerStatus> = self.manager_status.clone();
        let worker_status: Arc<WorkerStatus> = self.worker_status.clone();

        let worker = move || {
            Self::start_worker(
                &signals,
                &job_channel,
                &result_channel,
                &manager_status,
                &worker_status,
            );
            Self::set_active(&manager_status, &worker_status, false);
            signals.set_termination_signal(false);
        };
        worker
    }
}
