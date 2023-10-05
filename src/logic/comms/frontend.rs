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
    StreamToken { token: String, chat_name: String },
    DoneStreaming { chat_name: String },
    NewChatThread(String),
}

#[derive(Default, Debug, Clone)]
pub struct CurrentExchange {
    pub user_input_field: String,
    pub agent_responses: Vec<FrontendRequest>,
}

unsafe impl Send for FrontendRequest {}
unsafe impl Sync for FrontendRequest {}

impl FrontendComms {
    pub fn init(sender: FrontendSender, receiver: FrontendReceiver) -> Self {
        Self {
            sender: sender.into(),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}
