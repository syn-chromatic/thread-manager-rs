use std::sync::Mutex;

use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvError;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::SendError;
use std::sync::mpsc::Sender;
use std::sync::mpsc::TryRecvError;

use std::time::Duration;
use std::time::Instant;

use crate::status::ChannelStatus;

pub enum MessageKind<T> {
    Job(T),
    Release,
}

pub struct JobChannel<T> {
    sender: Sender<MessageKind<T>>,
    receiver: Mutex<Receiver<MessageKind<T>>>,
    status: ChannelStatus,
}

impl<T> JobChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (Sender<MessageKind<T>>, Receiver<MessageKind<T>>) = channel();

        let receiver: Mutex<Receiver<MessageKind<T>>> = Mutex::new(receiver);
        let status: ChannelStatus = ChannelStatus::new();

        Self {
            sender,
            receiver,
            status,
        }
    }

    pub fn status(&self) -> &ChannelStatus {
        &self.status
    }

    pub fn send(&self, value: T) -> Result<(), SendError<MessageKind<T>>> {
        self.status.add_sending();
        let message: MessageKind<T> = MessageKind::Job(value);
        let result: Result<(), SendError<MessageKind<T>>> = self.sender.send(message);
        self.status.sub_sending();
        if let Ok(_) = result {
            self.status.add_sent();
            return Ok(());
        }
        Err(result.unwrap_err())
    }

    pub fn send_timeout(
        &self,
        mut value: T,
        timeout: Duration,
    ) -> Result<(), SendError<MessageKind<T>>> {
        let now: Instant = Instant::now();
        while let Err(error) = self.send(value) {
            if now.elapsed() >= timeout {
                return Err(error);
            }
            match error.0 {
                MessageKind::Job(job) => value = job,
                MessageKind::Release => return Ok(()),
            }
        }
        Ok(())
    }

    pub fn send_release(&self) -> Result<(), SendError<MessageKind<T>>> {
        let message: MessageKind<T> = MessageKind::Release;
        let result: Result<(), SendError<MessageKind<T>>> = self.sender.send(message);
        if result.is_ok() {
            return Ok(());
        }
        Err(result.unwrap_err())
    }

    pub fn recv(&self) -> Result<MessageKind<T>, RecvError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(message) = receiver_guard.recv() {
                self.on_message_receive(&message);
                return Ok(message);
            }
        }
        self.status.sub_receiving();
        Err(RecvError)
    }

    pub fn try_recv(&self) -> Result<MessageKind<T>, TryRecvError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(message) = receiver_guard.try_recv() {
                self.on_message_receive(&message);
                return Ok(message);
            }
        }
        self.status.sub_receiving();
        Err(TryRecvError::Disconnected)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<MessageKind<T>, RecvTimeoutError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(message) = receiver_guard.recv_timeout(timeout) {
                self.on_message_receive(&message);
                return Ok(message);
            }
        }
        self.status.sub_receiving();
        Err(RecvTimeoutError::Disconnected)
    }

    pub fn is_finished(&self) -> bool {
        let status: &ChannelStatus = self.status();

        if status.concluded() != status.sent() {
            return false;
        }
        true
    }

    pub fn clear(&self) {
        while let Ok(value) = self.try_recv() {
            drop(value);
        }
    }
}

impl<T> JobChannel<T> {
    fn on_message_receive(&self, message: &MessageKind<T>) {
        match message {
            MessageKind::Job(_) => self.status.add_received(),
            MessageKind::Release => {}
        }
        self.status.sub_receiving();
    }
}

pub struct ResultChannel<T> {
    sender: Sender<T>,
    receiver: Mutex<Receiver<T>>,
    status: ChannelStatus,
}

impl<T> ResultChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (Sender<T>, Receiver<T>) = channel();

        let receiver: Mutex<Receiver<T>> = Mutex::new(receiver);
        let status: ChannelStatus = ChannelStatus::new();

        Self {
            sender,
            receiver,
            status,
        }
    }

    pub fn status(&self) -> &ChannelStatus {
        &self.status
    }

    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.status.add_sending();
        let result: Result<(), SendError<T>> = self.sender.send(value);
        self.status.sub_sending();
        if let Ok(_) = result {
            self.status.add_sent();
            return Ok(());
        }
        Err(result.unwrap_err())
    }

    pub fn send_timeout(&self, mut value: T, timeout: Duration) -> Result<(), SendError<T>> {
        let now: Instant = Instant::now();
        while let Err(error) = self.send(value) {
            if now.elapsed() >= timeout {
                return Err(error);
            }
            value = error.0;
        }
        Ok(())
    }

    pub fn recv(&self) -> Result<T, RecvError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(result) = receiver_guard.recv() {
                self.on_result_receive();
                return Ok(result);
            }
        }
        self.status.sub_receiving();
        Err(RecvError)
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(result) = receiver_guard.try_recv() {
                self.on_result_receive();
                return Ok(result);
            }
        }
        self.status.sub_receiving();
        Err(TryRecvError::Disconnected)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(result) = receiver_guard.recv_timeout(timeout) {
                self.on_result_receive();
                return Ok(result);
            }
        }
        self.status.sub_receiving();
        Err(RecvTimeoutError::Disconnected)
    }

    pub fn is_finished(&self) -> bool {
        let status: &ChannelStatus = self.status();

        if status.concluded() != status.sent() {
            return false;
        }
        true
    }

    pub fn clear(&self) {
        while let Ok(value) = self.try_recv() {
            drop(value);
        }
    }
}

impl<T> ResultChannel<T> {
    fn on_result_receive(&self) {
        self.status.add_received();
        self.status.sub_receiving();
    }
}
