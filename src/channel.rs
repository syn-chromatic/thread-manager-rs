use crossbeam_channel::unbounded;
use crossbeam_channel::Receiver;
use crossbeam_channel::RecvError;
use crossbeam_channel::RecvTimeoutError;
use crossbeam_channel::SendError;
use crossbeam_channel::Sender;
use crossbeam_channel::TryRecvError;

use std::time::Duration;
use std::time::Instant;

use crate::status::ChannelStatus;

pub enum MessageKind<T> {
    Job(T),
    Release,
}

pub struct JobChannel<T> {
    sender: Sender<MessageKind<T>>,
    receiver: Receiver<MessageKind<T>>,
    status: ChannelStatus,
}

impl<T> JobChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (Sender<MessageKind<T>>, Receiver<MessageKind<T>>) = unbounded();
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
        self.status.set_sending(true);
        let message: MessageKind<T> = MessageKind::Job(value);
        let result: Result<(), SendError<MessageKind<T>>> = self.sender.send(message);
        self.status.set_sending(false);
        if let Ok(_) = result {
            self.status.set_sent(true);
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
        self.status.set_receiving(true);

        if let Ok(message) = self.receiver.recv() {
            self.on_message_receive(&message);
            return Ok(message);
        }

        self.status.set_receiving(false);
        Err(RecvError)
    }

    pub fn try_recv(&self) -> Result<MessageKind<T>, TryRecvError> {
        self.status.set_receiving(true);

        if let Ok(message) = self.receiver.try_recv() {
            self.on_message_receive(&message);
            return Ok(message);
        }

        self.status.set_receiving(false);
        Err(TryRecvError::Disconnected)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<MessageKind<T>, RecvTimeoutError> {
        self.status.set_receiving(true);

        if let Ok(message) = self.receiver.recv_timeout(timeout) {
            self.on_message_receive(&message);
            return Ok(message);
        }

        self.status.set_receiving(false);
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
            MessageKind::Job(_) => self.status.set_received(true),
            MessageKind::Release => {}
        }
        self.status.set_receiving(false);
    }
}

pub struct ResultChannel<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
    status: ChannelStatus,
}

impl<T> ResultChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (Sender<T>, Receiver<T>) = unbounded();
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
        self.status.set_sending(true);
        let result: Result<(), SendError<T>> = self.sender.send(value);
        self.status.set_sending(false);
        if let Ok(_) = result {
            self.status.set_sent(true);
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
        self.status.set_receiving(true);

        if let Ok(result) = self.receiver.recv() {
            self.on_result_receive();
            return Ok(result);
        }

        self.status.set_receiving(false);
        Err(RecvError)
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.status.set_receiving(true);

        if let Ok(result) = self.receiver.try_recv() {
            self.on_result_receive();
            return Ok(result);
        }

        self.status.set_receiving(false);
        Err(TryRecvError::Disconnected)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        self.status.set_receiving(true);

        if let Ok(result) = self.receiver.recv_timeout(timeout) {
            self.on_result_receive();
            return Ok(result);
        }

        self.status.set_receiving(false);
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
        self.status.set_received(true);
        self.status.set_receiving(false);
    }
}
