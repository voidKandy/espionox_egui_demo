pub mod chat;
use super::comms::{backend::*, FrontendRequest};
use chat::{ChatAgentThread, ChatThreadVector};
use espionox::{
    agent::{Agent, AgentSettings},
    context::MessageVector,
};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

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
    pub agent_thread_names: Vec<String>,
    agent_threads: Arc<RwLock<ChatThreadVector>>,
    main_thread: Option<BackendThread>,
    sender: Arc<BackendSender>,
    receiver: Arc<Mutex<BackendCommandReceiver>>,
}

impl AppBackend {
    pub fn init(
        sender: mpsc::Sender<FrontendRequest>,
        receiver: mpsc::Receiver<BackendCommand>,
    ) -> Self {
        let sender = Arc::new(sender.into());
        let (agent_threads, agent_thread_names) =
            Self::create_default_agent_threads(Arc::clone(&sender));
        let agent_threads = Arc::new(agent_threads);
        let mut backend = Self {
            agent_thread_names,
            agent_threads,
            main_thread: None,
            sender: sender.into(),
            receiver: Arc::new(Mutex::new(receiver.into())),
        };
        backend
            .spawn_main_thread()
            .expect("Failed to spawn main backend thread");
        backend
    }

    fn create_default_agent_threads(
        sender: Arc<BackendSender>,
    ) -> (RwLock<ChatThreadVector>, Vec<String>) {
        let names = vec!["Default", "Default2"];
        let settings = AgentSettings::default();
        let agent_thread =
            ChatAgentThread::new(names[0], settings.clone(), Arc::clone(&sender)).unwrap();
        let agent_thread2 =
            ChatAgentThread::new(names[1], settings.clone(), Arc::clone(&sender)).unwrap();
        let agents = vec![agent_thread, agent_thread2];
        (
            RwLock::new(ChatThreadVector::from(agents)),
            names.iter().map(|n| n.to_string()).collect(),
        )
    }

    pub fn buffer(&self, agent: &Agent) -> anyhow::Result<Arc<MessageVector>> {
        let buffer = Arc::new(agent.context.buffer().to_owned());
        Ok(buffer)
    }

    pub fn spawn_main_thread(&mut self) -> Result<(), BackendError> {
        let receiver = Arc::clone(&self.receiver);
        let outer_sender = Arc::clone(&self.sender);
        let agent_threads = Arc::clone(&self.agent_threads);
        let handle = tokio::spawn(async move {
            loop {
                if let Some(command) = receiver
                    .try_lock()
                    .expect("Failed to lock receiver")
                    .receive_command()?
                {
                    match command {
                        BackendCommand::NewChatThread { name, settings } => {
                            tracing::info!("Received command to create new chat thread: {}", name);
                            let new_thread =
                                ChatAgentThread::new(&name, settings, Arc::clone(&outer_sender))
                                    .unwrap();

                            agent_threads.write().await.push(new_thread);
                            let frontend_request = FrontendRequest::NewChatThread(name);
                            outer_sender.send(frontend_request).await.map_err(|err| {
                                BackendError::Unexpected(anyhow::anyhow!(
                                    "Error sending command to agent thread: {:?}",
                                    err
                                ))
                            })?

                            // Create easy function for adding agents to threads list
                            // agent_threads_lock.insert(name, settings);
                            // self.agent_settings.insert(name, settings);
                            // Ok(())
                        }
                        BackendCommand::StreamedCompletion { agent_name, prompt } => {
                            let threads_lock = agent_threads.read().await;
                            let agent_thread = threads_lock
                                .get_by_name(&agent_name)
                                .expect("Failed to get chat thread");
                            if let Some(sender) = &agent_thread.sender {
                                tracing::info!("Trying to send prompt to {} agent", agent_name);
                                sender.send(prompt).await.map_err(|err| {
                                    BackendError::Unexpected(anyhow::anyhow!(
                                        "Error sending command to agent thread: {:?}",
                                        err
                                    ))
                                })?
                            } else {
                                tracing::warn!("Couldn't get sender from {} agent", agent_name);
                            }
                        }
                    };
                } else {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        });
        self.main_thread = Some(BackendThread::from(handle));
        Ok(())
    }
}
