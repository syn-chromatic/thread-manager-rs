use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use crossbeam_channel::RecvError;

use crate::channel::JobChannel;
use crate::channel::MessageKind;
use crate::channel::ResultChannel;

use crate::status::ManagerStatus;
use crate::status::WorkerStatus;

use crate::order::LOAD_ORDER;
use crate::order::STORE_ORDER;

pub struct ThreadWorker<F, T>
where
    F: Fn() -> T + Send + 'static,
{
    thread: Cell<Option<thread::JoinHandle<()>>>,
    worker: Arc<Worker<F, T>>,
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
        let worker: Arc<Worker<F, T>> =
            Worker::new(id, job_channel, result_channel, manager_status);

        Self { thread, worker }
    }

    pub fn start(&self) {
        if !self.worker.status().is_active() {
            self.worker.set_active(true);
            self.worker.set_termination(false);
            self.create_thread();
        }
    }

    pub fn send(&self, job: F) {
        self.worker.send_job(job);
        self.start();
    }
}

impl<F, T> ThreadWorker<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    pub fn id(&self) -> usize {
        self.worker.id()
    }

    pub fn status(&self) -> &WorkerStatus {
        self.worker.status()
    }

    pub fn job_channel(&self) -> &Arc<JobChannel<F>> {
        self.worker.job_channel()
    }

    pub fn join(&self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    pub fn set_termination(&self, state: bool) {
        self.worker.set_termination(state)
    }

    pub fn send_release(&self) {
        self.worker.send_release()
    }
}

impl<F, T> ThreadWorker<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    fn create_thread(&self) {
        let worker: Arc<Worker<F, T>> = self.worker.clone();
        let thread: thread::JoinHandle<()> = thread::spawn(worker.create());
        self.thread.set(Some(thread));
    }
}

pub struct Worker<F, T>
where
    F: Fn() -> T + Send + 'static,
{
    id: usize,
    termination: AtomicBool,
    job_channel: Arc<JobChannel<F>>,
    result_channel: Arc<ResultChannel<T>>,
    manager_status: Arc<ManagerStatus>,
    worker_status: WorkerStatus,
}

impl<F, T> Worker<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    pub fn new(
        id: usize,
        job_channel: Arc<JobChannel<F>>,
        result_channel: Arc<ResultChannel<T>>,
        manager_status: Arc<ManagerStatus>,
    ) -> Arc<Worker<F, T>> {
        let termination: AtomicBool = AtomicBool::new(false);
        let worker_status: WorkerStatus = WorkerStatus::new();

        Arc::new(Self {
            id,
            termination,
            job_channel,
            result_channel,
            manager_status,
            worker_status,
        })
    }

    pub fn id(self: &Arc<Self>) -> usize {
        self.id
    }

    pub fn send_job(self: &Arc<Self>, job: F) {
        self.job_channel
            .send(job)
            .expect(&format!("Failed to send job to Worker [{}]", self.id()));
    }

    pub fn create(self: Arc<Self>) -> impl Fn() {
        let worker = move || {
            self.worker();
            self.set_active(false);
            self.set_termination(false);
        };
        worker
    }

    pub fn status<'a>(self: &'a Arc<Self>) -> &'a WorkerStatus {
        &self.worker_status
    }

    pub fn job_channel<'a>(self: &'a Arc<Self>) -> &'a Arc<JobChannel<F>> {
        &self.job_channel
    }

    pub fn set_active(self: &Arc<Self>, state: bool) {
        self.worker_status.set_active(state);
        self.manager_status.set_active(state);
    }

    pub fn set_waiting(self: &Arc<Self>, state: bool) {
        self.worker_status.set_waiting(state);
        self.manager_status.set_waiting(state);
    }

    pub fn set_busy(self: &Arc<Self>, state: bool) {
        self.worker_status.set_busy(state);
        self.manager_status.set_busy(state);
    }

    pub fn set_termination(self: &Arc<Self>, state: bool) {
        self.termination.store(state, STORE_ORDER);
    }

    pub fn send_release(self: &Arc<Self>) {
        self.job_channel
            .send_release()
            .expect(&format!("Failed to release Worker [{}]", self.id()));
    }
}

impl<F, T> Worker<F, T>
where
    F: Fn() -> T + Send + 'static,
    T: Send + 'static,
{
    fn worker(self: &Arc<Self>) {
        while !self.termination.load(LOAD_ORDER) {
            let recv: Result<MessageKind<F>, RecvError> = self.wait_for_recv();

            if let Ok(message) = recv {
                match message {
                    MessageKind::Job(job) => {
                        self.job_recv(job);
                    }
                    MessageKind::Release => {
                        break;
                    }
                }
            }
        }
    }

    fn wait_for_recv(self: &Arc<Self>) -> Result<MessageKind<F>, RecvError> {
        self.set_waiting(true);
        let recv: Result<MessageKind<F>, RecvError> = self.job_channel.recv();
        self.set_waiting(false);
        recv
    }

    fn job_recv(self: &Arc<Self>, job: F) {
        self.worker_status.add_received();
        self.set_busy(true);

        self.result_channel
            .send(job())
            .expect("Failed to send result");

        self.job_channel.status().set_concluded(true);
        self.set_busy(false);
    }
}
