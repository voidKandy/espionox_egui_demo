use super::BackendCommand;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub type FrontendSender = mpsc::Sender<BackendCommand>;
pub type FrontendReceiver = mpsc::Receiver<FrontendRequest>;

#[derive(Debug)]
pub struct FrontendComms {
    pub sender: Arc<FrontendSender>,
    pub receiver: Arc<Mutex<FrontendReceiver>>,
}

#[derive(Debug, Clone)]
pub enum FrontendRequest {
    Message(String),
    DoneStreaming,
}

#[derive(Default, Debug, Clone)]
pub struct CurrentExchange {
    pub user_input_field: String,
    pub agent_responses: Vec<FrontendRequest>,
}

unsafe impl Send for FrontendRequest {}
unsafe impl Sync for FrontendRequest {}

impl From<String> for FrontendRequest {
    fn from(str: String) -> Self {
        Self::Message(str)
    }
}

impl Into<String> for FrontendRequest {
    fn into(self) -> String {
        match self {
            Self::Message(string) => string,
            Self::DoneStreaming => String::new(),
        }
    }
}

impl FrontendComms {
    pub fn init(sender: FrontendSender, receiver: FrontendReceiver) -> Self {
        Self {
            sender: sender.into(),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}
