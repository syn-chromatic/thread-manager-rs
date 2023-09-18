use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::status::ChannelStatus;

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

pub struct AtomicChannel<T> {
    sender: mpsc::Sender<ChannelMessage<T>>,
    receiver: Mutex<mpsc::Receiver<ChannelMessage<T>>>,
    status: ChannelStatus,
}

impl<T> AtomicChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (
            mpsc::Sender<ChannelMessage<T>>,
            mpsc::Receiver<ChannelMessage<T>>,
        ) = mpsc::channel();

        let receiver: Mutex<mpsc::Receiver<ChannelMessage<T>>> = Mutex::new(receiver);
        let status: ChannelStatus = ChannelStatus::new();

        AtomicChannel {
            sender,
            receiver,
            status,
        }
    }

    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<T>> {
        self.status.add_sending_count();
        let channel_message: ChannelMessage<T> = ChannelMessage::new(value, MessageKind::Value);
        let sent_result: Result<(), mpsc::SendError<ChannelMessage<T>>> =
            self.sender.send(channel_message);
        self.status.sub_sending_count();
        if let Ok(_) = sent_result {
            self.status.add_sent_count();
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
        self.status.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<ChannelMessage<T>, mpsc::RecvError> = receiver_guard.recv();
            if let Ok(channel_message) = received_result {
                match channel_message.kind {
                    MessageKind::Value => self.status.add_received_count(),
                    MessageKind::Release => {}
                }
                self.status.sub_receiving_count();
                return Ok((channel_message.value, channel_message.kind));
            }
        }
        self.status.sub_receiving_count();
        Err(mpsc::RecvError)
    }

    pub fn try_recv(&self) -> Result<(T, MessageKind), mpsc::TryRecvError> {
        self.status.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<ChannelMessage<T>, mpsc::TryRecvError> =
                receiver_guard.try_recv();
            if let Ok(channel_message) = received_result {
                match channel_message.kind {
                    MessageKind::Value => self.status.add_received_count(),
                    MessageKind::Release => {}
                }
                self.status.sub_receiving_count();
                return Ok((channel_message.value, channel_message.kind));
            }
        }
        self.status.sub_receiving_count();
        Err(mpsc::TryRecvError::Disconnected)
    }

    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<(T, MessageKind), mpsc::RecvTimeoutError> {
        self.status.add_receiving_count();
        if let Ok(receiver_guard) = self.receiver.lock() {
            let received_result: Result<ChannelMessage<T>, mpsc::RecvTimeoutError> =
                receiver_guard.recv_timeout(timeout);
            if let Ok(channel_message) = received_result {
                match channel_message.kind {
                    MessageKind::Value => self.status.add_received_count(),
                    MessageKind::Release => {}
                }
                self.status.sub_receiving_count();
                return Ok((channel_message.value, channel_message.kind));
            }
        }
        self.status.sub_receiving_count();
        Err(mpsc::RecvTimeoutError::Disconnected)
    }

    pub fn conclude(&self, kind: MessageKind) {
        match kind {
            MessageKind::Value => self.status.add_concluded_count(),
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
        self.status.get_sent_count()
    }

    pub fn get_sending_count(&self) -> usize {
        self.status.get_sending_count()
    }

    pub fn get_received_count(&self) -> usize {
        self.status.get_received_count()
    }

    pub fn get_receiving_count(&self) -> usize {
        self.status.get_receiving_count()
    }

    pub fn get_concluded_count(&self) -> usize {
        self.status.get_concluded_count()
    }

    pub fn clear_receiver(&self) {
        while let Ok(value) = self.try_recv() {
            drop(value);
        }
    }
}
