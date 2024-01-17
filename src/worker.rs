use std::sync::mpsc::RecvError;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use crate::types::Job;

use crate::channel::JobChannel;
use crate::channel::MessageKind;
use crate::channel::ResultChannel;

use crate::status::ManagerStatus;
use crate::status::WorkerSignals;
use crate::status::WorkerStatus;

pub struct ThreadWorker<T> {
    id: usize,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    signals: Arc<WorkerSignals>,
    job_channel: Arc<JobChannel<Job<T>>>,
    result_channel: Arc<ResultChannel<T>>,
    manager_status: Arc<ManagerStatus>,
    worker_status: Arc<WorkerStatus>,
}

impl<T: Send + 'static> ThreadWorker<T> {
    pub fn new(
        id: usize,
        job_channel: Arc<JobChannel<Job<T>>>,
        result_channel: Arc<ResultChannel<T>>,
        manager_status: Arc<ManagerStatus>,
    ) -> Self {
        let thread: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
        let signals: Arc<WorkerSignals> = Arc::new(WorkerSignals::new());
        let worker_status: Arc<WorkerStatus> = Arc::new(WorkerStatus::new());

        ThreadWorker {
            id,
            thread,
            signals,
            job_channel,
            result_channel,
            manager_status,
            worker_status,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn status(&self) -> &Arc<WorkerStatus> {
        &self.worker_status
    }

    pub fn start(&self) {
        if !self.worker_status.is_active() {
            self.spawn_thread();
        }
    }

    pub fn send(&self, job: Job<T>) {
        self.job_channel
            .send(job)
            .expect(&format!("Failed to send job to Worker [{}]", self.id()));
        self.start();
    }

    pub fn join(&self) {
        if let Ok(mut thread_guard) = self.thread.lock() {
            if let Some(thread) = thread_guard.take() {
                let _ = thread.join();
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        if let Ok(thread_guard) = self.thread.lock() {
            if let Some(thread) = thread_guard.as_ref() {
                let is_finished: bool = thread.is_finished();
                return is_finished;
            }
        }
        false
    }

    pub fn send_join_signal(&self) {
        self.signals.set_join_signal(true);
    }

    pub fn send_termination_signal(&self) {
        self.signals.set_termination_signal(true);
        self.send_release_signal();
    }

    pub fn send_release_signal(&self) {
        self.job_channel
            .send_release()
            .expect(&format!("Failed to release Worker [{}]", self.id()));
    }
}

impl<T: Send + 'static> ThreadWorker<T> {
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

    fn handle_job(
        job_channel: &Arc<JobChannel<Job<T>>>,
        result_channel: &Arc<ResultChannel<T>>,
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
    ) {
        Self::set_waiting(&manager_status, &worker_status, true);
        let recv: Result<MessageKind<Job<T>>, RecvError> = job_channel.recv();
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
                MessageKind::Release => {}
            }
        }
    }

    fn start_worker(
        signals: &Arc<WorkerSignals>,
        job_channel: &Arc<JobChannel<Job<T>>>,
        result_channel: &Arc<ResultChannel<T>>,
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
    ) {
        while !signals.termination_signal() {
            if signals.join_signal() {
                if job_channel.status().pending() == 0 {
                    break;
                }
            }
            Self::handle_job(job_channel, result_channel, manager_status, worker_status);
        }
    }

    fn create_worker(&self) -> impl Fn() {
        let signals: Arc<WorkerSignals> = self.signals.clone();
        let job_channel: Arc<JobChannel<Job<T>>> = self.job_channel.clone();
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

    fn spawn_thread(&self) {
        if let Ok(mut thread_guard) = self.thread.lock() {
            if let Some(thread) = thread_guard.take() {
                let _ = thread.join();
                let waiting: usize = self.manager_status.waiting_threads();
                let pending: usize = self.job_channel.status().pending();
                if waiting > pending {
                    return;
                }
            }

            Self::set_active(&self.manager_status, &self.worker_status, true);
            let thread: thread::JoinHandle<()> = thread::spawn(self.create_worker());
            *thread_guard = Some(thread);
        }
    }
}
