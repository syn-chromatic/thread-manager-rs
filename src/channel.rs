use std::sync::mpsc;
use std::sync::Mutex;

use std::time::Duration;
use std::time::Instant;

use crate::status::ChannelStatus;

pub enum MessageKind<T> {
    Job(T),
    Release,
}

pub struct JobChannel<T> {
    sender: mpsc::Sender<MessageKind<T>>,
    receiver: Mutex<mpsc::Receiver<MessageKind<T>>>,
    status: ChannelStatus,
}

impl<T> JobChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver): (mpsc::Sender<MessageKind<T>>, mpsc::Receiver<MessageKind<T>>) =
            mpsc::channel();

        let receiver: Mutex<mpsc::Receiver<MessageKind<T>>> = Mutex::new(receiver);
        let status: ChannelStatus = ChannelStatus::new();

        JobChannel {
            sender,
            receiver,
            status,
        }
    }

    pub fn status(&self) -> &ChannelStatus {
        &self.status
    }

    pub fn send(&self, value: T) -> Result<(), mpsc::SendError<MessageKind<T>>> {
        self.status.add_sending();
        let message: MessageKind<T> = MessageKind::Job(value);
        let result: Result<(), mpsc::SendError<MessageKind<T>>> = self.sender.send(message);
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
    ) -> Result<(), mpsc::SendError<MessageKind<T>>> {
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

    pub fn send_release(&self) -> Result<(), mpsc::SendError<MessageKind<T>>> {
        let message: MessageKind<T> = MessageKind::Release;
        let result: Result<(), mpsc::SendError<MessageKind<T>>> = self.sender.send(message);
        if let Ok(_) = result {
            return Ok(());
        }
        Err(result.unwrap_err())
    }

    pub fn recv(&self) -> Result<MessageKind<T>, mpsc::RecvError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(message) = receiver_guard.recv() {
                self.on_message_receive(&message);
                return Ok(message);
            }
        }
        self.status.sub_receiving();
        Err(mpsc::RecvError)
    }

    pub fn try_recv(&self) -> Result<MessageKind<T>, mpsc::TryRecvError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(message) = receiver_guard.try_recv() {
                self.on_message_receive(&message);
                return Ok(message);
            }
        }
        self.status.sub_receiving();
        Err(mpsc::TryRecvError::Disconnected)
    }

    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<MessageKind<T>, mpsc::RecvTimeoutError> {
        self.status.add_receiving();
        if let Ok(receiver_guard) = self.receiver.lock() {
            if let Ok(message) = receiver_guard.recv_timeout(timeout) {
                self.on_message_receive(&message);
                return Ok(message);
            }
        }
        self.status.sub_receiving();
        Err(mpsc::RecvTimeoutError::Disconnected)
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
