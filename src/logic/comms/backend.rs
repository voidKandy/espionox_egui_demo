use super::FrontendRequest;
use crate::backend::BackendError;
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinHandle,
};

#[derive(Clone, Debug)]
pub enum BackendCommand {
    StreamedCompletion { agent_name: String, prompt: String },
}

unsafe impl Send for BackendCommand {}
unsafe impl Sync for BackendCommand {}

pub type BackendSender = Sender<FrontendRequest>;

#[derive(Debug)]
pub struct BackendCompletionThread(JoinHandle<Result<(), BackendError>>);

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

impl From<JoinHandle<Result<(), BackendError>>> for BackendCompletionThread {
    fn from(value: JoinHandle<Result<(), BackendError>>) -> Self {
        Self(value)
    }
}

impl AsRef<JoinHandle<Result<(), BackendError>>> for BackendCompletionThread {
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

    // pub async fn get_completion_receiver(
    //     &mut self,
    //     agent: &mut Agent,
    // ) -> Result<CompletionReceiverHandler, BackendError> {
    //     match self.receive_command() {
    //         Ok(command) => {
    //             let prompt: String = command.into();
    //             Ok(agent
    //                 .stream_prompt(prompt)
    //                 .await
    //                 .expect("Failed to get receiver"))
    //         }
    //         Err(err) => Err(err),
    //     }
    // }
}

// impl BackendCommand {
//     pub fn is_empty(&self) -> bool {
//         match self {
//             Self::StreamedCompletion(prompt) => prompt.is_empty(),
//             Self::ChangeCurrentAgent(name) => name.is_empty(),
//         }
//     }
// }
