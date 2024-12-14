use bytes::Bytes;
use cpu_time::ThreadTime;
use std::ops::{AddAssign, Sub};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

use crate::data_manager;
use crate::pleiades_type::{Job, JobStatus};

/// Contractor
///
///
///
///
pub struct PendingManager {
    /// interface to access this component
    ///
    command_receiver: mpsc::Receiver<Command>,

    /// updater
    ///
    data_manager_controller: data_manager::Controller,
}

impl PendingManager {
    /// new
    ///
    ///
    ///
    ///
    pub fn new(data_manager_controller: data_manager::Controller) -> (Self, Controller) {
        let (command_sender, command_receiver) = mpsc::channel(64);

        let data_manager = Self {
            command_receiver,
            data_manager_controller,
        };

        let controller = Controller { command_sender };

        (data_manager, controller)
    }

    /// run
    ///
    ///
    ///
    ///
    pub async fn run(&mut self) {
        while let Some(request) = self.command_receiver.recv().await {
            match request {
                Command::Enqueue(request) => {},
            }
        }
    }
}

/// Api
///
///
///
///
#[derive(Clone)]
pub struct Controller {
    command_sender: mpsc::Sender<Command>,
}

impl Controller {
    /// get_blob
    ///
    ///
    ///
    ///
    pub async fn register(&self, job: Job) {
        let request = Command::Enqueue(register::Request { job });
        self.command_sender.send(request).await.unwrap();
    }
}

/// Command
///
///
///
///
pub enum Command {
    Enqueue(register::Request),
}

pub mod register {
    use super::*;

    pub struct Request {
        pub job: Job,
    }
}
