use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct AtomicChannel<T> {
    pub sender: Mutex<mpsc::Sender<T>>,
    pub receiver: Mutex<mpsc::Receiver<T>>,
    buffer: AtomicUsize,
}

impl<T> AtomicChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel();
        let sender: Mutex<mpsc::Sender<T>> = Mutex::new(sender);
        let receiver: Mutex<mpsc::Receiver<T>> = Mutex::new(receiver);
        let buffer: AtomicUsize = AtomicUsize::new(0);

        AtomicChannel {
            sender,
            receiver,
            buffer,
        }
    }

    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        if let Ok(sender_guard) = self.sender.lock() {
            let sent_result: Result<(), mpsc::SendError<T>> = sender_guard.send(value);
            if sent_result.is_ok() {
                self.buffer.fetch_add(1, Ordering::Release);
            }
            return sent_result;
        }
        Err(mpsc::SendError(value))
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
                self.buffer.fetch_sub(1, Ordering::Release);
            }
            return received_result;
        }
        Err(mpsc::RecvError)
    }

    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<T, mpsc::TryRecvError> = receiver_guard.try_recv();
            if received_result.is_ok() {
                self.buffer.fetch_sub(1, Ordering::Release);
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
                self.buffer.fetch_sub(1, Ordering::Release);
            }
            return received_result;
        }
        Err(mpsc::RecvTimeoutError::Disconnected)
    }

    pub fn get_buffer(&self) -> usize {
        let receive_buffer: usize = self.buffer.load(Ordering::Acquire);
        receive_buffer
    }

    pub fn clear_receiver(&self) {
        while let Ok(value) = self.try_recv() {
            drop(value);
        }
    }
}
