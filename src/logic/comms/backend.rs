use super::FrontendRequest;
use crate::backend::BackendError;
use espionox::{agents::Agent, memory::Message};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

#[derive(Clone, Debug)]
pub enum BackendCommand {
    StreamedCompletion {
        agent_name: String,
        prompt: String,
    },
    PushToAgentMemory {
        agent_name: String,
        message: Message,
    },
    NewChatThread {
        name: String,
        agent: Agent,
    },
    RemoveChatThread {
        name: String,
    },
}

unsafe impl Send for BackendCommand {}
unsafe impl Sync for BackendCommand {}

pub type BackendSender = Sender<FrontendRequest>;

#[derive(Debug)]
pub struct BackendThread(JoinHandle<Result<(), BackendError>>);

#[derive(Debug)]
pub struct BackendCommandReceiver(Receiver<BackendCommand>);

impl From<Receiver<BackendCommand>> for BackendCommandReceiver {
    fn from(value: Receiver<BackendCommand>) -> Self {
        Self(value)
    }
}

impl AsRef<Receiver<BackendCommand>> for BackendCommandReceiver {
    fn as_ref(&self) -> &Receiver<BackendCommand> {
        &self.0
    }
}

impl AsMut<Receiver<BackendCommand>> for BackendCommandReceiver {
    fn as_mut(&mut self) -> &mut Receiver<BackendCommand> {
        &mut self.0
    }
}

impl From<JoinHandle<Result<(), BackendError>>> for BackendThread {
    fn from(value: JoinHandle<Result<(), BackendError>>) -> Self {
        Self(value)
    }
}

impl AsRef<JoinHandle<Result<(), BackendError>>> for BackendThread {
    fn as_ref(&self) -> &JoinHandle<Result<(), BackendError>> {
        &self.0
    }
}

impl BackendCommandReceiver {
    pub fn receive_command(&mut self) -> Result<Option<BackendCommand>, BackendError> {
        tracing::info!("Listening for backend command...");
        match self.as_mut().try_recv() {
            Ok(command) => {
                println!("Command received: {:?}", command);
                Ok(Some(command))
            }
            Err(err) => match err {
                mpsc::error::TryRecvError::Empty => Ok(None),
                _ => Err(anyhow::anyhow!("{:?}", err).into()),
            },
        }
    }
}
