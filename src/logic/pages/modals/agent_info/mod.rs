mod components;
use components::*;

use crate::logic::{comms::BackendCommand, ChatPage, FrontendComms};
use eframe::{
    egui::{self, TextEdit},
    epaint::Color32,
};
use espionox::{
    agents::Agent,
    language_models::LanguageModel,
    memory::{
        long_term::LongTermMemory, CachingMechanism, Memory, Message, MessageVector, RecallMode,
    },
};
use std::{any, cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct AgentInfoModal {
    chat_name: String,
    open: OpenOptions,
    init_prompt_ui: Rc<RefCell<InitPromptUi>>,
    recall_mode: RecallMode,
    caching_mechanism_ui: CachingMechanismUi,
    long_term_memory: LongTermMemory,
    pub error_message: Option<String>,
}

#[derive(Debug)]
struct OpenOptions {
    system_prompt: bool,
    recall_mode: bool,
    caching_mechanism: bool,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            system_prompt: false,
            recall_mode: false,
            caching_mechanism: false,
        }
    }
}

impl TryInto<BackendCommand> for &mut AgentInfoModal {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<BackendCommand, Self::Error> {
        let name = self.chat_name.to_string();
        if name.trim().is_empty() {
            self.error_message = Some("Name cannot be empty".to_string());
            return Err(anyhow::anyhow!("Name is empty"));
        }
        let memory = Memory::build()
            .caching_mechanism(self.caching_mechanism_ui.caching_mechanism().clone())
            .recall(self.recall_mode.clone())
            .init_prompt(self.init_prompt_ui.borrow().init_prompt().clone())
            .finished();
        let model = LanguageModel::default_gpt();
        let agent = Agent { memory, model };
        Ok(BackendCommand::NewChatThread { name, agent })
    }
}

impl AgentInfoModal {
    pub fn new_empty() -> Self {
        let prompt = MessageVector::from_message(Message::new_standard(
            espionox::memory::MessageRole::System,
            &String::new(),
        ));
        let init_prompt_ui = Rc::new(RefCell::new(prompt.into()));
        Self {
            chat_name: String::new(),
            open: OpenOptions::default(),
            init_prompt_ui,
            recall_mode: RecallMode::default(),
            caching_mechanism_ui: CachingMechanism::default().into(),
            long_term_memory: LongTermMemory::None,
            error_message: None,
        }
    }

    pub fn from(agent: &Agent, name: &str) -> Self {
        let mut prompt = agent.memory.cache().clone();
        prompt.reset_to_system_prompt();
        let init_prompt_ui = Rc::new(RefCell::new(prompt.into()));
        Self {
            chat_name: name.to_string(),
            open: OpenOptions::default(),
            init_prompt_ui,
            recall_mode: agent.memory.recall_mode().clone(),
            caching_mechanism_ui: agent.memory.caching_mechanism().clone().into(),
            long_term_memory: LongTermMemory::None,
            error_message: None,
        }
    }

    // fn init_prompt_ui(&mut self, ui: &mut egui::Ui) {
    //     ui.vertical_centered(|ui| {
    //         let prompt_ui_rc = Rc::clone(&self.init_prompt_ui);
    //         InitPromptUi::display_message_input(ui, &prompt_ui_rc);
    //         InitPromptUi::display_buttons(ui, &prompt_ui_rc);
    //     });
    // }

    fn recall_mode(&mut self, ui: &mut egui::Ui) {
        let current_mode = &self.recall_mode;
        ui.colored_label(
            Color32::KHAKI,
            format!("Current recall mode: {:?}", current_mode),
        );
    }

    pub fn display_agent_form(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::TextEdit::singleline(&mut self.chat_name).hint_text("New chat name"));

        if ui
            .selectable_label(self.open.system_prompt, "Init Prompt")
            .clicked()
        {
            self.open.system_prompt = !self.open.system_prompt;
        }
        if self.open.system_prompt {
            self.open.recall_mode = false;
            self.open.caching_mechanism = false;
            InitPromptUi::overview_display(Rc::clone(&self.init_prompt_ui), ui);
        }

        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.open.recall_mode, "Recall")
                .clicked()
            {
                self.open.recall_mode = !self.open.recall_mode;
            }
            ui.colored_label(Color32::GOLD, format!("{:?}", &self.recall_mode));
        });

        if self.open.recall_mode {
            self.open.caching_mechanism = false;
            self.open.system_prompt = false;
            self.recall_mode(ui);
        }

        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.open.caching_mechanism, "Caching")
                .clicked()
            {
                self.open.caching_mechanism = !self.open.caching_mechanism;
            }
            ui.colored_label(Color32::GOLD, &self.caching_mechanism_ui.mech_name());
        });

        if self.open.caching_mechanism {
            self.open.recall_mode = false;
            self.open.system_prompt = false;
            self.caching_mechanism_ui.overview_display(ui);
        }
    }
}
