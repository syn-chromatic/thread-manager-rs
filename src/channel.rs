use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct AtomicChannel<T> {
    sender: mpsc::Sender<T>,
    receiver: Mutex<mpsc::Receiver<T>>,
    pending_count: Arc<AtomicUsize>,
    receive_count: Arc<AtomicUsize>,
}

impl<T> AtomicChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel();
        let receiver: Mutex<mpsc::Receiver<T>> = Mutex::new(receiver);
        let pending_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let receive_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        AtomicChannel {
            sender,
            receiver,
            pending_count,
            receive_count,
        }
    }

    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        let sent_result: Result<(), mpsc::SendError<T>> = self.sender.send(value);
        if sent_result.is_ok() {
            self.pending_count.fetch_add(1, Ordering::Release);
        }
        sent_result
    }

    pub fn send_timeout(&self, mut value: T, timeout: Duration) -> Result<(), mpsc::SendError<T>> {
        let now: Instant = Instant::now();
        while let Err(error) = self.send(value) {
            if now.elapsed() >= timeout {
                return Err(error);
            }
            value = error.0;
        }
        Ok(())
    }

    pub fn recv(&self) -> Result<T, mpsc::RecvError> {
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<T, mpsc::RecvError> = receiver_guard.recv();
            if received_result.is_ok() {
                self.pending_count.fetch_sub(1, Ordering::Release);
                self.receive_count.fetch_add(1, Ordering::Release);
            }
            return received_result;
        }
        Err(mpsc::RecvError)
    }

    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<T, mpsc::TryRecvError> = receiver_guard.try_recv();
            if received_result.is_ok() {
                self.pending_count.fetch_sub(1, Ordering::Release);
                self.receive_count.fetch_add(1, Ordering::Release);
            }
            return received_result;
        }
        Err(mpsc::TryRecvError::Disconnected)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, mpsc::RecvTimeoutError> {
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<T, mpsc::RecvTimeoutError> =
                receiver_guard.recv_timeout(timeout);
            if received_result.is_ok() {
                self.pending_count.fetch_sub(1, Ordering::Release);
                self.receive_count.fetch_add(1, Ordering::Release);
            }
            return received_result;
        }
        Err(mpsc::RecvTimeoutError::Disconnected)
    }

    pub fn get_pending_count(&self) -> usize {
        let buffer: usize = self.pending_count.load(Ordering::Acquire);
        buffer
    }

    pub fn get_receive_count(&self) -> usize {
        let received: usize = self.receive_count.load(Ordering::Acquire);
        received
    }

    pub fn clear_receiver(&self) {
        while let Ok(value) = self.try_recv() {
            drop(value);
        }
    }
}
