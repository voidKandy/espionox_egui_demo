pub mod chat;
use super::comms::{backend::*, FrontendRequest};
use chat::{ChatAgentThread, ChatThreadVector};
use espionox::{
    agent::{Agent, AgentSettings},
    configuration::ConfigEnv,
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
    // pub agent_thread_names: Vec<String>,
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
        let agent_threads = Self::init_default_agent_threads(Arc::clone(&sender))
            .expect("Failed to init default threads");
        let agent_threads = Arc::new(agent_threads);
        let mut backend = Self {
            // agent_thread_names,
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

    fn init_default_agent_threads(
        sender: Arc<BackendSender>,
    ) -> Result<RwLock<ChatThreadVector>, BackendError> {
        let names = vec!["Chat Agent", "Long Term Agent"];
        let st_settings = AgentSettings::default();
        let st_agent_thread =
            ChatAgentThread::new(names[0], st_settings.to_owned(), Arc::clone(&sender));

        let lt_settings = AgentSettings::new()
            .long_term_env(ConfigEnv::default(), Some("EguiLongTerm"))
            .short_term(espionox::context::short_term::ShortTermMemory::new_cache())
            .build_buffer_from(espionox::context::MemoryVariant::LongTerm)
            .finish();
        let lt_agent_thread =
            ChatAgentThread::new(names[1], lt_settings.to_owned(), Arc::clone(&sender));

        let agents = vec![st_agent_thread, lt_agent_thread];

        for name in names.iter() {
            let frontend_request = FrontendRequest::NewChatThread(name.to_string());
            sender.try_send(frontend_request).map_err(|err| {
                BackendError::Unexpected(anyhow::anyhow!(
                    "Error sending command to agent thread: {:?}",
                    err
                ))
            })?
        }
        Ok(RwLock::new(ChatThreadVector::from(agents)))
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
                agent_threads
                    .read()
                    .await
                    .spawn_threads_if_handleless()
                    .await?;
                if let Some(command) = receiver
                    .try_lock()
                    .expect("Failed to lock receiver")
                    .receive_command()?
                {
                    match command {
                        BackendCommand::NewChatThread { name, settings } => {
                            tracing::info!("Received command to create new chat thread: {}", name);
                            let new_thread =
                                ChatAgentThread::new(&name, settings, Arc::clone(&outer_sender));
                            agent_threads.write().await.push(new_thread);
                            let frontend_request = FrontendRequest::NewChatThread(name);
                            outer_sender.send(frontend_request).await.map_err(|err| {
                                BackendError::Unexpected(anyhow::anyhow!(
                                    "Error sending command to agent thread: {:?}",
                                    err
                                ))
                            })?
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
                        BackendCommand::RemoveChatThread { name } => {
                            tracing::info!("Removing {} agent thread", name);
                            agent_threads.write().await.remove_by_name(&name);
                        }
                    };
                } else {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        });
        self.main_thread = Some(BackendThread::from(handle));
        Ok(())
    }
}
