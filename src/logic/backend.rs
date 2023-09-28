use super::comms::{backend::*, FrontendRequest};
use espionox::{agent::Agent, agent::CompletionReceiverHandler, context::MessageVector};
use std::sync::Arc;
use tokio::{
    runtime::Handle,
    sync::{mpsc, Mutex},
};

#[derive(thiserror::Error, Debug)]
pub enum BackendError {
    Recoverable,
    Unexpected(#[from] anyhow::Error),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Recoverable => {
                write!(f, "Recoverable backend error")
            }
            Self::Unexpected(err) => {
                write!(f, "Unexpect backend error: {:?}", err)
            }
        }
    }
}

#[derive(Debug)]
pub struct AppBackend {
    agent: Arc<Mutex<Agent>>,
    agent_thread: Option<BackendCompletionThread>,
    sender: Arc<BackendSender>,
    receiver: Arc<Mutex<BackendCommandReceiver>>,
}

impl AppBackend {
    pub fn init(
        sender: mpsc::Sender<FrontendRequest>,
        receiver: mpsc::Receiver<BackendCommand>,
    ) -> Self {
        Self {
            agent: Arc::new(Mutex::new(Agent::default())),
            agent_thread: None,
            sender: sender.into(),
            receiver: Arc::new(Mutex::new(receiver.into())),
        }
    }

    pub fn agent_thread(&self) -> bool {
        self.agent_thread.is_some()
    }

    pub fn buffer(&self) -> anyhow::Result<Arc<MessageVector>> {
        let agent_lock = self.agent.try_lock().unwrap();
        let buffer = Arc::new(agent_lock.context.buffer().to_owned());
        Ok(buffer)
    }

    #[tracing::instrument(name = "Spawn agent thread")]
    pub fn spawn_agent_thread(&mut self) -> anyhow::Result<()> {
        let receiver = Arc::clone(&self.receiver);
        let sender = Arc::clone(&self.sender);
        let agent = Arc::clone(&self.agent);
        let handle = tokio::spawn(async move {
            println!("Ok we here");
            let mut receiver_lock = receiver.lock().await;
            let mut agent_lock = agent.lock().await;
            loop {
                match receiver_lock.receive_command()? {
                    Some(command) => {
                        let prompt: String = command.into();
                        println!("Prompt: {}", prompt);
                        let response = agent_lock
                            .prompt(prompt)
                            .await
                            .expect("Failed to prompt agent");
                        println!("response: {}", response);
                        let _ = sender
                            .send(FrontendRequest::Message(response.to_owned()))
                            .await;
                        // std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                    None => {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                }
            }
        });
        self.agent_thread = Some(BackendCompletionThread::from(handle));
        Ok(())
    }

    #[tracing::instrument(name = "Spawn completion thread")]
    pub fn spawn_completion_thread(&mut self) -> anyhow::Result<()> {
        let receiver = Arc::clone(&self.receiver);
        let sender = Arc::clone(&self.sender);
        let agent = Arc::clone(&self.agent);
        let handle = tokio::spawn(async move {
            let mut receiver_lock = receiver.lock().await;
            let mut agent_lock = agent.lock().await;
            let mut full_message = vec![];
            loop {
                match receiver_lock.receive_command()? {
                    Some(command) => {
                        let prompt: String = command.into();
                        let mut stream_receiver = agent_lock
                            .stream_prompt(prompt)
                            .await
                            .map_err(|err| BackendError::Unexpected(err.into()))?;

                        while let Ok(Some(token)) = stream_receiver.receive().await {
                            tracing::info!("Token got: {}", token);
                            sender.send(token.to_owned().into()).await.unwrap();
                            full_message.push(token.to_owned());
                        }
                        agent_lock
                            .context
                            .push_to_buffer("assistant", full_message.join(""));
                        full_message.clear();
                        tracing::info!("Processed all responses");
                    }
                    None => {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                }
            }
        });
        self.agent_thread = Some(BackendCompletionThread::from(handle));
        Ok(())
    }
}
