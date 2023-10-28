use espionox::{
    agents::Agent,
    language_models::LanguageModel,
    memory::{Memory, Message},
};

use super::{BackendError, BackendSender};
use crate::logic::comms::FrontendRequest;
use std::sync::Arc;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

#[derive(Debug)]
pub struct ChatAgentThread {
    handle: Option<JoinHandle<()>>,
    pub name: String,
    agent_construct: Option<AgentConstructor>,
    pub sender: Option<mpsc::Sender<ChatAgentMutation>>,
    outer_sender: Arc<BackendSender>,
    // receiver: Option<mpsc::Receiver<String>>,
}

pub enum ChatAgentMutation {
    Prompt(String),
    PushMessage(Message),
}

#[derive(Debug, Clone)]
struct AgentConstructor {
    memory: Memory,
    model: LanguageModel,
}

impl From<Agent> for AgentConstructor {
    fn from(value: Agent) -> Self {
        let memory = value.memory;
        let model = value.model;
        Self { memory, model }
    }
}

impl Into<Agent> for AgentConstructor {
    fn into(self) -> Agent {
        Agent {
            memory: self.memory,
            model: self.model,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ChatThreadVector(Vec<Arc<Mutex<ChatAgentThread>>>);

impl ChatAgentThread {
    pub fn new(name: &str, agent: Agent, outer_sender: Arc<BackendSender>) -> Self {
        let agent_thread = ChatAgentThread {
            handle: None,
            name: name.to_string(),
            agent_construct: Some(agent.into()),
            sender: None,
            outer_sender,
        };
        agent_thread
    }

    pub fn close(&mut self) {
        self.handle = None;
        self.sender = None;
    }

    pub fn has_handle(&self) -> bool {
        self.handle.is_some() && self.sender.is_some()
    }

    #[tracing::instrument(name = "Spawn completion thread")]
    pub fn spawn_chat_thread(&mut self) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::channel::<ChatAgentMutation>(5);
        self.sender = Some(tx);
        tracing::info!("Set sender for {} agent thread", self.name);
        let outer_sender = Arc::clone(&self.outer_sender);
        let chat_name = self.name.to_string();
        let agent_const = self.agent_construct.take();
        let handle = tokio::spawn(async move {
            let mut agent = agent_const.unwrap().into();
            loop {
                tracing::info!("Listening on {} agent thread...", &chat_name);
                match rx.try_recv() {
                    Ok(mutation) => match mutation {
                        ChatAgentMutation::Prompt(prompt) => {
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
                        ChatAgentMutation::PushMessage(message) => {
                            tracing::info!("Received message on agent thread");
                            agent.memory.force_push_message_to_cache(message);
                        }
                    },
                    Err(err) => match err {
                        tokio::sync::mpsc::error::TryRecvError::Empty => {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                        tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                            tracing::warn!("{} thread Disconnected", chat_name);
                            break;
                        }
                    },
                }
            }
        });
        self.handle = Some(handle);
        tracing::info!("Set handle for {} agent thread", self.name);
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
            .memory
            .push_to_message_cache(Some("assistant"), full_message.join(""))
            .await;
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
    pub fn remove_by_name(&mut self, thread_name: &str) {
        self.as_mut().retain(|thread_mutex| {
            let mut thread = thread_mutex.try_lock().unwrap();
            if thread.name == thread_name {
                // Stop the thread before removing it
                thread.close();
                drop(thread);
                false
            } else {
                true
            }
        });
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

    fn handleless_indices(&self) -> Vec<usize> {
        self.0
            .iter()
            .enumerate()
            .filter(|(_, th)| !th.try_lock().unwrap().has_handle())
            .map(|(index, _)| index)
            .collect()
    }
    pub async fn spawn_threads_if_handleless(&self) -> anyhow::Result<()> {
        let indices = self.handleless_indices();
        tracing::info!(
            "{} out of {} threads are handleless",
            indices.len(),
            self.0.len()
        );
        for ix in indices {
            let mut thread = self.0[ix].lock().await;
            tracing::info!("Spawning {} thread...", thread.name);
            if !thread.has_handle() {
                thread.spawn_chat_thread()?;
            }
        }
        Ok(())
    }
}
