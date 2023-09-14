use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const LOAD_ORDER: Ordering = Ordering::Acquire;
const FETCH_ORDER: Ordering = Ordering::Release;

pub struct AtomicChannel<T> {
    sender: mpsc::Sender<(T, bool)>,
    receiver: Mutex<mpsc::Receiver<(T, bool)>>,
    sent_count: Arc<AtomicUsize>,
    sending_count: Arc<AtomicUsize>,
    received_count: Arc<AtomicUsize>,
    receiving_count: Arc<AtomicUsize>,
}

impl<T> AtomicChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (mpsc::Sender<(T, bool)>, mpsc::Receiver<(T, bool)>) =
            mpsc::channel();
        let receiver: Mutex<mpsc::Receiver<(T, bool)>> = Mutex::new(receiver);
        let sent_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let sending_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let received_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let receiving_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        AtomicChannel {
            sender,
            receiver,
            sent_count,
            sending_count,
            received_count,
            receiving_count,
        }
    }

    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        self.add_sending_count();
        let sent_result: Result<(), mpsc::SendError<(T, bool)>> = self.sender.send((value, true));
        self.sub_sending_count();
        if let Ok(_) = sent_result {
            self.add_sent_count();
            return Ok(());
        }
        let error: mpsc::SendError<(T, bool)> = sent_result.unwrap_err();
        Err(mpsc::SendError::<T>(error.0 .0))
    }

    pub fn send_uncounted(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        let sent_result: Result<(), mpsc::SendError<(T, bool)>> = self.sender.send((value, false));
        if let Ok(_) = sent_result {
            return Ok(());
        }
        let error: mpsc::SendError<(T, bool)> = sent_result.unwrap_err();
        Err(mpsc::SendError::<T>(error.0 .0))
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
        self.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<(T, bool), mpsc::RecvError> = receiver_guard.recv();
            if let Ok((received, boolean)) = received_result {
                if boolean {
                    self.add_received_count();
                }
                self.sub_receiving_count();
                return Ok(received);
            }
        }
        self.sub_receiving_count();
        Err(mpsc::RecvError)
    }

    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<(T, bool), mpsc::TryRecvError> = receiver_guard.try_recv();
            if let Ok((received, boolean)) = received_result {
                if boolean {
                    self.add_received_count();
                }
                self.sub_receiving_count();
                return Ok(received);
            }
        }
        self.sub_receiving_count();
        Err(mpsc::TryRecvError::Disconnected)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, mpsc::RecvTimeoutError> {
        self.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<(T, bool), mpsc::RecvTimeoutError> =
                receiver_guard.recv_timeout(timeout);
            if let Ok((received, boolean)) = received_result {
                if boolean {
                    self.add_received_count();
                }
                self.sub_receiving_count();
                return Ok(received);
            }
        }
        self.sub_receiving_count();
        Err(mpsc::RecvTimeoutError::Disconnected)
    }

    pub fn get_pending_count(&self) -> usize {
        let sent_count: usize = self.get_sent_count();
        let sending_count: usize = self.get_sending_count();
        let received_count: usize = self.get_received_count();
        (sent_count + sending_count) - received_count
    }

    pub fn get_available_count(&self) -> usize {
        let sent_count: usize = self.get_sent_count();
        let received_count: usize = self.get_received_count();
        sent_count - received_count
    }

    pub fn get_sent_count(&self) -> usize {
        let sent_count: usize = self.sent_count.load(LOAD_ORDER);
        sent_count
    }

    pub fn get_sending_count(&self) -> usize {
        let sending_count: usize = self.sending_count.load(LOAD_ORDER);
        sending_count
    }

    pub fn get_received_count(&self) -> usize {
        let received_count: usize = self.received_count.load(LOAD_ORDER);
        received_count
    }

    pub fn get_receiving_count(&self) -> usize {
        let receiving_count: usize = self.receiving_count.load(LOAD_ORDER);
        receiving_count
    }

    pub fn clear_receiver(&self) {
        while let Ok(value) = self.try_recv() {
            drop(value);
        }
    }
}

impl<T> AtomicChannel<T> {
    fn add_sent_count(&self) {
        self.sent_count.fetch_add(1, FETCH_ORDER);
    }

    fn add_sending_count(&self) {
        self.sending_count.fetch_add(1, FETCH_ORDER);
    }

    fn sub_sending_count(&self) {
        self.sending_count.fetch_sub(1, FETCH_ORDER);
    }

    fn add_received_count(&self) {
        self.received_count.fetch_add(1, FETCH_ORDER);
    }

    fn add_receiving_count(&self) {
        self.receiving_count.fetch_add(1, FETCH_ORDER);
    }

    fn sub_receiving_count(&self) {
        self.receiving_count.fetch_sub(1, FETCH_ORDER);
    }
}
