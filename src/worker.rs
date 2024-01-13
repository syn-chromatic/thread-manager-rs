use std::sync::{Arc, Mutex};
use std::thread;

use crate::channel::AtomicChannel;
use crate::signals::WorkerSignals;
use crate::status::{ManagerStatus, WorkerStatus};
use crate::types::Job;

pub struct ThreadWorker {
    id: usize,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    channel: Arc<AtomicChannel<Job>>,
    signals: Arc<WorkerSignals>,
    manager_status: Arc<ManagerStatus>,
    worker_status: Arc<WorkerStatus>,
}

impl ThreadWorker {
    pub fn new(
        id: usize,
        channel: Arc<AtomicChannel<Job>>,
        manager_status: Arc<ManagerStatus>,
    ) -> Self {
        let thread: Mutex<Option<thread::JoinHandle<()>>> = Mutex::new(None);
        let signals: Arc<WorkerSignals> = Arc::new(WorkerSignals::new());
        let worker_status: Arc<WorkerStatus> = Arc::new(WorkerStatus::new());

        ThreadWorker {
            id,
            thread,
            channel,
            signals,
            manager_status,
            worker_status,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn start(&self) {
        if !self.is_active() {
            self.handle_spawn_thread();
        }
    }

    pub fn send<F>(&self, function: F)
    where
        F: Fn() + Send + 'static,
    {
        let job: Job = Box::new(function);
        self.channel
            .send(job)
            .expect(&format!("Failed to send job to Worker [{}]", self.id()));
        self.start();
    }

    pub fn join(&self) {
        if let Ok(mut thread_option) = self.thread.lock() {
            if let Some(thread) = thread_option.take() {
                let _ = thread.join();
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        if let Ok(thread_option) = self.thread.lock() {
            if let Some(thread) = thread_option.as_ref() {
                let is_finished: bool = thread.is_finished();
                return is_finished;
            }
        }
        false
    }

    pub fn is_active(&self) -> bool {
        self.worker_status.get_is_active()
    }

    pub fn is_busy(&self) -> bool {
        self.worker_status.get_is_busy()
    }

    pub fn is_waiting(&self) -> bool {
        self.worker_status.get_is_waiting()
    }

    pub fn get_received_jobs(&self) -> usize {
        self.worker_status.get_received_jobs()
    }

    pub fn send_join_signal(&self) {
        self.signals.set_join_signal(true);
    }

    pub fn send_termination_signal(&self) {
        self.signals.set_termination_signal(true);
        self.send_channel_release();
    }

    pub fn send_channel_release(&self) {
        let closure: Job = Box::new(move || {});
        self.channel
            .send_release(Box::new(closure))
            .expect(&format!("Failed to release Worker [{}]", self.id()));
    }
}

impl ThreadWorker {
    fn handle_spawn_thread(&self) {
        if let Ok(mut thread_guard) = self.thread.lock() {
            if let Some(existing_thread) = thread_guard.take() {
                let _ = existing_thread.join();
                let pending_count: usize = self.channel.get_pending_count();
                let waiting_threads: usize = self.manager_status.get_waiting_threads();
                if waiting_threads > pending_count {
                    return;
                }
            }

            let manager_status: Arc<ManagerStatus> = self.manager_status.clone();
            Self::set_broad_active_state(&manager_status, &self.worker_status, true);

            let worker_loop = self.create_worker_loop();
            let thread: thread::JoinHandle<()> = thread::spawn(worker_loop);
            *thread_guard = Some(thread);
        }
    }

    fn set_broad_active_state(
        manager_status: &Arc<ManagerStatus>,
        status: &Arc<WorkerStatus>,
        state: bool,
    ) {
        status.set_active_state(state);
        match state {
            true => manager_status.add_active_threads(),
            false => manager_status.sub_active_threads(),
        }
    }

    fn set_broad_waiting_state(
        manager_status: &Arc<ManagerStatus>,
        status: &Arc<WorkerStatus>,
        state: bool,
    ) {
        status.set_waiting_state(state);
        match state {
            true => manager_status.add_waiting_threads(),
            false => manager_status.sub_waiting_threads(),
        }
    }

    fn set_broad_busy_state(
        manager_status: &Arc<ManagerStatus>,
        status: &Arc<WorkerStatus>,
        state: bool,
    ) {
        status.set_busy_state(state);
        match state {
            true => manager_status.add_busy_threads(),
            false => manager_status.sub_busy_threads(),
        }
    }

    fn handle_job(
        channel: &Arc<AtomicChannel<Job>>,
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
    ) {
        Self::set_broad_waiting_state(&manager_status, &worker_status, true);
        let recv = channel.recv();
        Self::set_broad_waiting_state(&manager_status, &worker_status, false);
        if let Ok((job, kind)) = recv {
            worker_status.add_received_job();
            Self::set_broad_busy_state(&manager_status, &worker_status, true);
            job();
            Self::set_broad_busy_state(&manager_status, &worker_status, false);
            channel.conclude(kind);
        }
    }

    fn initiate_worker_loop(
        channel: &Arc<AtomicChannel<Job>>,
        manager_status: &Arc<ManagerStatus>,
        worker_status: &Arc<WorkerStatus>,
        signals: &Arc<WorkerSignals>,
    ) {
        while !signals.get_termination_signal() {
            if signals.get_join_signal() {
                let pending_jobs: usize = channel.get_pending_count();
                if pending_jobs == 0 {
                    break;
                }
            }
            Self::handle_job(channel, manager_status, worker_status);
        }
    }

    fn create_worker_loop(&self) -> impl Fn() {
        let channel: Arc<AtomicChannel<Job>> = self.channel.clone();
        let manager_status: Arc<ManagerStatus> = self.manager_status.clone();
        let worker_status: Arc<WorkerStatus> = self.worker_status.clone();
        let signals: Arc<WorkerSignals> = self.signals.clone();

        let worker_loop = move || {
            Self::initiate_worker_loop(&channel, &manager_status, &worker_status, &signals);
            Self::set_broad_active_state(&manager_status, &worker_status, false);
            signals.set_termination_signal(false);
        };
        worker_loop
    }
}
