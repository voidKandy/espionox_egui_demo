use super::comms::{backend::*, FrontendRequest};
use espionox::{
    agent::{Agent, AgentSettings},
    context::MessageVector,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};

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
    agent_settings: HashMap<String, AgentSettings>,
    chat_threads: HashMap<String, Option<BackendCompletionThread>>,
    sender: Arc<BackendSender>,
    receiver: Arc<Mutex<BackendCommandReceiver>>,
}

impl AppBackend {
    pub fn init(
        sender: mpsc::Sender<FrontendRequest>,
        receiver: mpsc::Receiver<BackendCommand>,
    ) -> Self {
        let agent_settings = Self::create_default_agents();
        Self {
            agent_settings,
            chat_threads: HashMap::new(),
            sender: sender.into(),
            receiver: Arc::new(Mutex::new(receiver.into())),
        }
    }

    fn create_default_agents() -> HashMap<String, AgentSettings> {
        let mut agents = HashMap::new();
        let agent1 = AgentSettings::default();
        let agent2 = AgentSettings::default();
        agents.insert("Default".to_string(), agent1);
        agents.insert("Default2".to_string(), agent2);
        agents
    }

    pub fn all_agent_names(&self) -> Vec<String> {
        self.agent_settings.keys().cloned().collect::<Vec<String>>()
    }

    pub fn max_chat_threads_spawned(&self) -> bool {
        self.chat_threads.len() == self.all_agent_names().len()
    }

    pub fn buffer(&self, agent: &Agent) -> anyhow::Result<Arc<MessageVector>> {
        let buffer = Arc::new(agent.context.buffer().to_owned());
        Ok(buffer)
    }

    #[tracing::instrument(name = "Spawn completion thread")]
    pub fn spawn_chat_threads(&mut self) -> anyhow::Result<()> {
        for name in self.all_agent_names() {
            let receiver = Arc::clone(&self.receiver);
            let sender = Arc::clone(&self.sender);
            let settings = self.agent_settings.get(&name).unwrap().clone();
            let handle = tokio::spawn(async move {
                let mut receiver_lock = receiver.lock().await;
                let mut agent = Agent::build(settings).expect("Failed to build agent");
                loop {
                    if let Some(command) = receiver_lock.receive_command()? {
                        match command {
                            BackendCommand::StreamedCompletion { prompt, .. } => {
                                Self::handle_completion_stream(
                                    prompt,
                                    &mut agent,
                                    Arc::clone(&sender),
                                )
                                .await
                                .unwrap();
                            }
                        }
                    } else {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                }
            });
            self.chat_threads
                .insert(name, Some(BackendCompletionThread::from(handle)));
        }
        Ok(())
    }

    async fn handle_completion_stream(
        prompt: String,
        agent: &mut Agent,
        sender: Arc<BackendSender>,
    ) -> Result<(), BackendError> {
        let mut stream_receiver = agent
            .stream_prompt(prompt)
            .await
            .map_err(|err| BackendError::Unexpected(err.into()))?;
        let mut full_message = vec![];
        while let Ok(Some(token)) = stream_receiver.receive().await {
            tracing::info!("token got: {}", token);
            sender.send(token.to_owned().into()).await.unwrap();
            full_message.push(token.to_owned());
        }
        agent
            .context
            .push_to_buffer("assistant", full_message.join(""));
        full_message.clear();

        sender.send(FrontendRequest::DoneStreaming).await.unwrap();
        tracing::info!("processed all responses");
        Ok(())
    }
    // #[tracing::instrument(name = "Spawn completion thread")]
    // pub fn spawn_completion_thread(&mut self, agent_name: &str) -> anyhow::Result<()> {
    //     self.switch_current_agent(agent_name)
    //         .expect("Failed to switch agent");
    //     let receiver = Arc::clone(&self.receiver);
    //     let sender = Arc::clone(&self.sender);
    //     let (_, agent_mutex) = self.current_agent.as_ref().expect("No current agent");
    //     let agent_mutex_cloned = Arc::clone(agent_mutex);
    //     let handle = tokio::spawn(async move {
    //         let mut receiver_lock = receiver.lock().await;
    //         let mut agent_lock = agent_mutex_cloned.lock().await;
    //         let mut full_message = vec![];
    //         loop {
    //             match receiver_lock.receive_command()? {
    //                 Some(command) => {
    //                     let prompt: String = command.into();
    //                     let mut stream_receiver = agent_lock
    //                         .stream_prompt(prompt)
    //                         .await
    //                         .map_err(|err| BackendError::Unexpected(err.into()))?;
    //
    //                     while let Ok(Some(token)) = stream_receiver.receive().await {
    //                         tracing::info!("Token got: {}", token);
    //                         sender.send(token.to_owned().into()).await.unwrap();
    //                         full_message.push(token.to_owned());
    //                     }
    //                     agent_lock
    //                         .context
    //                         .push_to_buffer("assistant", full_message.join(""));
    //                     full_message.clear();
    //
    //                     sender.send(FrontendRequest::DoneStreaming).await.unwrap();
    //                     tracing::info!("Processed all responses");
    //                 }
    //                 None => {
    //                     std::thread::sleep(std::time::Duration::from_secs(2));
    //                 }
    //             }
    //         }
    //     });
    //     self.completion_thread = Some(BackendCompletionThread::from(handle));
    //     Ok(())
    // }
}
