use espionox::agent::{Agent, AgentSettings};

use super::{BackendError, BackendSender};
use crate::comms::FrontendRequest;
use std::sync::Arc;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

#[derive(Debug)]
pub(super) struct ChatAgentThread {
    pub(super) handle: Option<JoinHandle<()>>,
    pub(super) name: String,
    pub(super) settings: AgentSettings,
    pub(super) sender: Option<mpsc::Sender<String>>,
    pub(super) outer_sender: Arc<BackendSender>,
}

#[derive(Debug, Clone)]
pub(super) struct ChatThreadVector(Vec<Arc<Mutex<ChatAgentThread>>>);

impl ChatAgentThread {
    #[tracing::instrument(name = "Spawn completion thread")]
    pub fn spawn_chat_thread(&mut self) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel::<String>(5);
        self.sender = Some(tx);
        let outer_sender = Arc::clone(&self.outer_sender);
        let mut agent = Agent::build(self.settings.to_owned()).expect("Failed to build agent");
        let handle = tokio::spawn(async move {
            loop {
                tracing::info!("Listening for prompt on agent thread...");
                match rx.try_recv() {
                    Ok(prompt) => {
                        tracing::info!("Prompt received on agent thread...");
                        Self::handle_completion_stream(
                            prompt,
                            &mut agent,
                            Arc::clone(&outer_sender),
                        )
                        .await
                        .unwrap();
                    }
                    // Ok(None) => {
                    //     tracing::info!("Prompt not received on agent thread...");
                    //     std::thread::sleep(std::time::Duration::from_secs(2));
                    // }
                    Err(err) => {
                        tracing::warn!("Error when trying to receive in agent thread: {:?}", err);
                        // tracing::info!("Prompt not received on agent thread...");
                        std::thread::sleep(std::time::Duration::from_secs(2));
                    }
                }
            }
        });
        self.handle = Some(handle);
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
}

impl From<Vec<ChatAgentThread>> for ChatThreadVector {
    fn from(value: Vec<ChatAgentThread>) -> Self {
        let threads: Vec<_> = value
            .into_iter()
            .map(|thread| Arc::new(Mutex::new(thread)))
            .collect();
        Self(threads)
    }
}

impl ChatThreadVector {
    pub fn get_by_name(&self, name: &str) -> Option<tokio::sync::MutexGuard<'_, ChatAgentThread>> {
        self.0
            .iter()
            .find(|thread_mutex| {
                let thread = thread_mutex.try_lock().unwrap();
                thread.name == name
            })
            .map(|thread_mutex| thread_mutex.try_lock().unwrap())
    }
}
