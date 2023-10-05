use espionox::agent::{Agent, AgentSettings};

use super::{BackendError, BackendSender};
use crate::logic::comms::{BackendCommand, FrontendRequest};
use std::sync::Arc;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

#[derive(Debug)]
pub struct ChatAgentThread {
    handle: Option<JoinHandle<()>>,
    pub name: String,
    settings: AgentSettings,
    pub sender: Option<mpsc::Sender<String>>,
    outer_sender: Arc<BackendSender>,
    // receiver: Option<mpsc::Receiver<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct ChatThreadVector(Vec<Arc<Mutex<ChatAgentThread>>>);

impl ChatAgentThread {
    pub fn new(
        name: &str,
        settings: AgentSettings,
        outer_sender: Arc<BackendSender>,
    ) -> Result<Self, BackendError> {
        let mut agent_thread = ChatAgentThread {
            handle: None,
            name: name.to_string(),
            settings,
            sender: None,
            outer_sender,
        };
        agent_thread.spawn_chat_thread()?;
        Ok(agent_thread)
    }

    #[tracing::instrument(name = "Spawn completion thread")]
    pub fn spawn_chat_thread(&mut self) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel::<String>(5);
        self.sender = Some(tx);
        let outer_sender = Arc::clone(&self.outer_sender);
        let mut agent = Agent::build(self.settings.to_owned()).expect("Failed to build agent");
        tracing::info!("Listening on {} agent thread...", self.name);
        let chat_name = self.name.to_string();
        let handle = tokio::spawn(async move {
            loop {
                match rx.try_recv() {
                    Ok(prompt) => {
                        tracing::info!("Prompt received on {} agent thread...", chat_name);
                        Self::handle_completion_stream(
                            chat_name.clone(),
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
                        if err == tokio::sync::mpsc::error::TryRecvError::Empty {
                            std::thread::sleep(std::time::Duration::from_secs(2));
                        } else {
                            tracing::warn!(
                                "Error when trying to receive in {} agent thread: {:?}",
                                chat_name,
                                err
                            );
                        }
                        // tracing::info!("Prompt not received on agent thread...");
                    }
                }
            }
        });
        self.handle = Some(handle);
        Ok(())
    }

    async fn handle_completion_stream(
        chat_name: String,
        prompt: String,
        agent: &mut Agent,
        sender: Arc<BackendSender>,
    ) -> Result<(), BackendError> {
        let mut stream_receiver = agent
            .stream_prompt(prompt)
            .await
            .map_err(|err| BackendError::Unexpected(err.into()))?;
        let mut full_message = vec![];
        while let Ok(Some(token_response)) = stream_receiver.receive().await {
            tracing::info!("Sending Token: {}", token_response);
            let token = token_response.to_owned();
            let chat_name = chat_name.to_owned();
            sender
                .send(FrontendRequest::StreamToken { token, chat_name })
                .await
                .unwrap();
            full_message.push(token_response.to_owned());
        }
        agent
            .context
            .push_to_buffer("assistant", full_message.join(""));
        full_message.clear();

        sender
            .send(FrontendRequest::DoneStreaming { chat_name })
            .await
            .unwrap();
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

impl AsMut<Vec<Arc<Mutex<ChatAgentThread>>>> for ChatThreadVector {
    fn as_mut(&mut self) -> &mut Vec<Arc<Mutex<ChatAgentThread>>> {
        &mut self.0
    }
}

impl ChatThreadVector {
    pub fn push(&mut self, thread: ChatAgentThread) {
        self.as_mut().push(Arc::new(Mutex::new(thread)));
    }
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
