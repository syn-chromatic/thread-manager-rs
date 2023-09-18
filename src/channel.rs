use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const LOAD_ORDER: Ordering = Ordering::Acquire;
const FETCH_ORDER: Ordering = Ordering::Release;

pub enum MessageKind {
    Value,
    Release,
}

struct ChannelMessage<T> {
    value: T,
    kind: MessageKind,
}

impl<T> ChannelMessage<T> {
    fn new(value: T, kind: MessageKind) -> Self {
        ChannelMessage { value, kind }
    }
}

struct ChannelCounters {
    sent_count: Arc<AtomicUsize>,
    sending_count: Arc<AtomicUsize>,
    received_count: Arc<AtomicUsize>,
    receiving_count: Arc<AtomicUsize>,
    concluded_count: Arc<AtomicUsize>,
}

impl ChannelCounters {
    fn new() -> Self {
        let sent_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let sending_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let received_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let receiving_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let concluded_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        ChannelCounters {
            sent_count,
            sending_count,
            received_count,
            receiving_count,
            concluded_count,
        }
    }

    fn get_sent_count(&self) -> usize {
        let sent_count: usize = self.sent_count.load(LOAD_ORDER);
        sent_count
    }

    fn get_sending_count(&self) -> usize {
        let sending_count: usize = self.sending_count.load(LOAD_ORDER);
        sending_count
    }

    fn get_received_count(&self) -> usize {
        let received_count: usize = self.received_count.load(LOAD_ORDER);
        received_count
    }

    fn get_receiving_count(&self) -> usize {
        let receiving_count: usize = self.receiving_count.load(LOAD_ORDER);
        receiving_count
    }

    fn get_concluded_count(&self) -> usize {
        let concluded_count: usize = self.concluded_count.load(LOAD_ORDER);
        concluded_count
    }

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

    fn add_concluded_count(&self) {
        self.concluded_count.fetch_add(1, FETCH_ORDER);
    }
}

pub struct AtomicChannel<T> {
    sender: mpsc::Sender<ChannelMessage<T>>,
    receiver: Mutex<mpsc::Receiver<ChannelMessage<T>>>,
    counters: ChannelCounters,
}

impl<T> AtomicChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (
            mpsc::Sender<ChannelMessage<T>>,
            mpsc::Receiver<ChannelMessage<T>>,
        ) = mpsc::channel();

        let receiver: Mutex<mpsc::Receiver<ChannelMessage<T>>> = Mutex::new(receiver);
        let counters: ChannelCounters = ChannelCounters::new();

        AtomicChannel {
            sender,
            receiver,
            counters,
        }
    }

    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        self.counters.add_sending_count();
        let channel_message: ChannelMessage<T> = ChannelMessage::new(value, MessageKind::Value);
        let sent_result: Result<(), mpsc::SendError<ChannelMessage<T>>> =
            self.sender.send(channel_message);
        self.counters.sub_sending_count();
        if let Ok(_) = sent_result {
            self.counters.add_sent_count();
            return Ok(());
        }
        let error: mpsc::SendError<ChannelMessage<T>> = sent_result.unwrap_err();
        Err(mpsc::SendError::<T>(error.0.value))
    }

    pub fn send_release(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        let channel_message: ChannelMessage<T> = ChannelMessage::new(value, MessageKind::Release);
        let sent_result: Result<(), mpsc::SendError<ChannelMessage<T>>> =
            self.sender.send(channel_message);
        if let Ok(_) = sent_result {
            return Ok(());
        }
        let error: mpsc::SendError<ChannelMessage<T>> = sent_result.unwrap_err();
        Err(mpsc::SendError::<T>(error.0.value))
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

    pub fn recv(&self) -> Result<(T, MessageKind), mpsc::RecvError> {
        self.counters.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<ChannelMessage<T>, mpsc::RecvError> = receiver_guard.recv();
            if let Ok(channel_message) = received_result {
                match channel_message.kind {
                    MessageKind::Value => self.counters.add_received_count(),
                    MessageKind::Release => {}
                }
                self.counters.sub_receiving_count();
                return Ok((channel_message.value, channel_message.kind));
            }
        }
        self.counters.sub_receiving_count();
        Err(mpsc::RecvError)
    }

    pub fn try_recv(&self) -> Result<(T, MessageKind), mpsc::TryRecvError> {
        self.counters.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<ChannelMessage<T>, mpsc::TryRecvError> =
                receiver_guard.try_recv();
            if let Ok(channel_message) = received_result {
                match channel_message.kind {
                    MessageKind::Value => self.counters.add_received_count(),
                    MessageKind::Release => {}
                }
                self.counters.sub_receiving_count();
                return Ok((channel_message.value, channel_message.kind));
            }
        }
        self.counters.sub_receiving_count();
        Err(mpsc::TryRecvError::Disconnected)
    }

    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<(T, MessageKind), mpsc::RecvTimeoutError> {
        self.counters.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<ChannelMessage<T>, mpsc::RecvTimeoutError> =
                receiver_guard.recv_timeout(timeout);
            if let Ok(channel_message) = received_result {
                match channel_message.kind {
                    MessageKind::Value => self.counters.add_received_count(),
                    MessageKind::Release => {}
                }
                self.counters.sub_receiving_count();
                return Ok((channel_message.value, channel_message.kind));
            }
        }
        self.counters.sub_receiving_count();
        Err(mpsc::RecvTimeoutError::Disconnected)
    }

    pub fn conclude(&self, kind: MessageKind) {
        match kind {
            MessageKind::Value => self.counters.add_concluded_count(),
            MessageKind::Release => {}
        }
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
        let sent_count: usize = self.counters.get_sent_count();
        sent_count
    }

    pub fn get_sending_count(&self) -> usize {
        let sending_count: usize = self.counters.get_sending_count();
        sending_count
    }

    pub fn get_received_count(&self) -> usize {
        let received_count: usize = self.counters.get_received_count();
        received_count
    }

    pub fn get_receiving_count(&self) -> usize {
        let receiving_count: usize = self.counters.get_receiving_count();
        receiving_count
    }

    pub fn get_concluded_count(&self) -> usize {
        let concluded_count: usize = self.counters.get_concluded_count();
        concluded_count
    }

    pub fn clear_receiver(&self) {
        while let Ok(value) = self.try_recv() {
            drop(value);
        }
    }
}
