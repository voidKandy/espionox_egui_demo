use super::ChatPage;
use crate::logic::FrontendComms;
use eframe::{
    egui::{self, Button, Id, Layout},
    epaint::{text::LayoutJob, Color32, Fonts, Galley},
    glow::NUM_COMPRESSED_TEXTURE_FORMATS,
};
use espionox::{
    agent::Agent,
    context::memory::{
        long_term::LongTermMemory, CachingMechanism, Message, MessageRole, MessageVector,
        RecallMode,
    },
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct AgentInfoModal {
    chat_name: String,
    init_prompt_ui: Rc<RefCell<InitPromptUi>>,
    recall_mode: RecallMode,
    caching_mechanism: CachingMechanism,
    long_term_memory: LongTermMemory,
    error_message: Option<String>,
}

#[derive(Debug)]
pub struct InitPromptUi {
    messages: MessageVector,
    current_message_idx: usize,
    current_message_content: String,
}

impl From<MessageVector> for InitPromptUi {
    fn from(mut value: MessageVector) -> Self {
        if value.len() == 0 {
            value.push(Message::new_standard(MessageRole::System, &String::new()));
        }
        let current_message_idx = 0;
        let current_message_content = value.as_ref()[current_message_idx]
            .content()
            .unwrap()
            .to_string();
        Self {
            messages: value,
            current_message_idx,
            current_message_content,
        }
    }
}

impl InitPromptUi {
    fn current_message(&mut self) -> &mut Message {
        &mut self.messages.as_mut()[self.current_message_idx]
    }
    fn current_message_role(&mut self) -> String {
        self.current_message().role().to_string()
    }
    fn save_current(&mut self) {
        let current = Message::new_standard(
            self.current_message_role().into(),
            &self.current_message_content,
        );
        self.messages.as_mut()[self.current_message_idx] = current.to_owned();
    }
    fn change_idx(&mut self, change: i8) {
        if change.is_negative() {
            if self.current_message_idx == 0 {
                return;
            }
        }
        self.save_current();
        self.current_message_idx = (self.current_message_idx as i8 + change) as usize;
        self.current_message_content = self.messages.as_ref()[self.current_message_idx]
            .content()
            .unwrap()
            .to_string();
    }
    fn has_next(&self) -> bool {
        if self.current_message_idx == self.messages.len() - 1 {
            return false;
        }
        self.messages
            .as_ref()
            .get(self.current_message_idx + 1)
            .is_some()
    }
    fn has_last(&self) -> bool {
        if self.current_message_idx == 0 {
            return false;
        }
        self.messages
            .as_ref()
            .get(self.current_message_idx - 1)
            .is_some()
    }
}

impl AgentInfoModal {
    pub fn new_empty() -> Self {
        let prompt = MessageVector::from_message(Message::new_standard(
            espionox::context::memory::MessageRole::System,
            &String::new(),
        ));
        let init_prompt_ui = Rc::new(RefCell::new(prompt.into()));
        Self {
            chat_name: String::new(),
            init_prompt_ui,
            recall_mode: RecallMode::default(),
            caching_mechanism: CachingMechanism::default(),
            long_term_memory: LongTermMemory::None,
            error_message: None,
        }
    }
    pub fn from(agent: Agent, name: &str) -> Self {
        let mut prompt = agent.memory.cache().clone();
        prompt.reset_to_system_prompt();
        let init_prompt_ui = Rc::new(RefCell::new(prompt.into()));
        Self {
            chat_name: name.to_string(),
            init_prompt_ui,
            recall_mode: agent.memory.recall_mode().clone(),
            caching_mechanism: agent.memory.caching_mechanism().clone(),
            long_term_memory: LongTermMemory::None,
            error_message: None,
        }
    }

    fn display_buttons(&self, ui: &mut egui::Ui, prompt_ui_rc: &Rc<RefCell<InitPromptUi>>) {
        ui.vertical_centered(|ui| {
            ui.columns(3, |col| {
                let mut prompt_ui = prompt_ui_rc.borrow_mut();
                let empty_message = Message::new_standard(
                    espionox::context::memory::MessageRole::System,
                    &String::new(),
                );

                // let last_button_string = LayoutJob {
                //     text: "⬅︎".to_string(),
                //     halign: eframe::emath::Align::Center,
                //     ..Default::default()
                // };
                // let next_button_string = LayoutJob {
                //     text: "➡︎".to_string(),
                //     halign: eframe::emath::Align::Center,
                //     ..Default::default()
                // };
                // let new_button_string = LayoutJob {
                //     text: "new".to_string(),
                //     halign: eframe::emath::Align::Center,
                //     ..Default::default()
                // };

                let last_button = Button::new("<");
                let next_button = Button::new(">");
                let mut has_last = &prompt_ui.has_last();
                col[0].add_enabled_ui(*has_last, |ui| {
                    if ui.add(last_button).clicked() {
                        prompt_ui.change_idx(-1);
                    }
                });

                if col[1].button("+").clicked() {
                    prompt_ui.messages.push(empty_message.clone());
                }

                let mut has_next = &prompt_ui.has_next();
                col[2].add_enabled_ui(*has_next, |ui| {
                    if ui.add(next_button).clicked() {
                        prompt_ui.change_idx(1);
                    }
                });
            });
        });
    }

    fn display_message_input(&self, ui: &mut egui::Ui, prompt_ui_rc: &Rc<RefCell<InitPromptUi>>) {
        ui.vertical_centered(|ui| {
            let mut prompt_ui = prompt_ui_rc.borrow_mut();
            let role_label = format!("Role: {}", prompt_ui.current_message_role());

            ui.label(role_label);

            let textedit = egui::TextEdit::multiline(&mut prompt_ui.current_message_content);

            ui.add(textedit);
        });
    }

    fn init_prompt_ui(&mut self, ui: &mut egui::Ui) {
        // ui.horizontal(|ui| {
        let prompt_ui_rc = Rc::clone(&self.init_prompt_ui);
        self.display_message_input(ui, &prompt_ui_rc);
        self.display_buttons(ui, &prompt_ui_rc);
        // });
    }

    fn recall_mode(&mut self, ui: &mut egui::Ui) {
        unimplemented!();
    }

    fn caching_mechanism(&mut self, ui: &mut egui::Ui) {
        let mut mech = &self.caching_mechanism;

        match mech {
            CachingMechanism::Forgetful => ui.label("Forgetful"),
            CachingMechanism::SummarizeAtLimit { save_to_lt, .. } => {
                ui.label("SummarizeAtLimit");
                ui.radio(*save_to_lt, "Save to long term")
            }
        };
        ui.label(format!(
            "Cache message limit (excluding system messages): {}",
            mech.limit()
        ));
    }

    pub fn display_form(&mut self, ui: &mut egui::Ui, frontend: &FrontendComms) {
        let chat_name = &self.chat_name;
        ui.with_layout(Layout::left_to_right(eframe::emath::Align::Min), |ui| {
            ui.colored_label(Color32::KHAKI, chat_name);
        });
        ui.vertical_centered(|ui| {
            self.init_prompt_ui(ui);
            // self.recall_mode(ui);
            // self.caching_mechanism(ui);
        });
    }
}
