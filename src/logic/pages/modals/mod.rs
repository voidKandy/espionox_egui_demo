use espionox::{
    agent::Agent,
    context::memory::{long_term::LongTermMemory, CachingMechanism, MessageVector, RecallMode},
};

use super::ChatPage;

#[derive(Debug, Clone)]
pub struct AgentInfoModal {
    chat_name: String,
    init_prompt: MessageVector,
    recall_mode: RecallMode,
    caching_mechanism: CachingMechanism,
    long_term_memory: LongTermMemory,
    error_message: Option<String>,
}

impl Default for AgentInfoModal {
    fn default() -> Self {
        Self {
            chat_name: String::new(),
            init_prompt: MessageVector::init(),
            recall_mode: RecallMode::default(),
            caching_mechanism: CachingMechanism::default(),
            long_term_memory: LongTermMemory::None,
            error_message: None,
        }
    }
}

impl From<Agent> for AgentInfoModal {
    fn from(agent: Agent) -> Self {
       let init_prompt = agent.memory.cache().reset_to_system_prompt() 
    }
}

impl AgentInfoModal {
    fn clear(&mut self) {
        *self = Self::default();
    }
}

impl AgentInfoModal {
    fn settings(&self) -> AgentSettings {
        AgentSettings::new()
            .short_term(self.selected_stm.clone())
            .init_prompt(MessageVector::init_with_system_prompt(&self.init_prompt))
            // .long_term(self.selected_ltm)
            .finish()
    }
}

impl ChatPage {
    pub fn display_modal(&mut self, ui: &mut egui::Ui, frontend: &FrontendComms) {
        let existing_names = self.all_chat_names();
        let modal = &mut self.create_chat_modal;

        let mut create_new_chat_modal_open = self.create_new_chat_modal_open;
        egui::Window::new("Create New Chats")
            .open(&mut create_new_chat_modal_open)
            .resizable(false)
            .movable(false)
            .collapsible(false)
            .anchor(Align2::RIGHT_TOP, [-5.0, -5.0])
            .show(ui.ctx(), |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut modal.chat_name)
                        .hint_text("Put a chat name here"),
                );
                ui.label("Short Term Memory");
                ui.radio_value(&mut modal.selected_stm, ShortTermMemory::Forget, "None");
                ui.radio_value(
                    &mut modal.selected_stm,
                    ShortTermMemory::Cache(MemoryCache::default()),
                    "Cached Memory",
                );
                ui.label("Long Term Memory");
                ui.radio_value(&mut modal.selected_ltm, LongTermMemory::None, "None");
                let init_prompt_te = egui::TextEdit::multiline(&mut modal.init_prompt)
                    .hint_text("Put your desired system prompt here");
                let _ = ui.add(init_prompt_te);
                if modal.error_message.is_some() {
                    ui.colored_label(Color32::RED, modal.error_message.as_ref().unwrap());
                }
                if ui.button("Create New Chat").clicked() {
                    if modal.chat_name.trim().is_empty() {
                        modal.error_message = Some("Please fill out the name field".to_string());
                    } else if existing_names.contains(&modal.chat_name) {
                        modal.error_message =
                            Some("Cannot create duplicate chat names".to_string());
                    } else {
                        let name = &modal.chat_name;
                        let name = name.to_string();
                        let settings = modal.settings();
                        let create_command = BackendCommand::NewChatThread { name, settings };
                        frontend
                            .sender
                            .try_send(create_command)
                            .expect("Failed to send chat creation command");
                        modal.clear();
                        self.create_new_chat_modal_open = false;
                    }
                }
            });
    }
}
