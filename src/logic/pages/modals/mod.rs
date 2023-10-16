use super::ChatPage;
use crate::logic::FrontendComms;
use eframe::{
    egui::{self, Button, Id, Layout, RichText},
    epaint::{text::LayoutJob, Color32, FontId, Fonts, Galley},
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
    open: OpenOptions,
    init_prompt_ui: Rc<RefCell<InitPromptUi>>,
    recall_mode: RecallMode,
    caching_mechanism_ui: CachingMechanismUi,
    long_term_memory: LongTermMemory,
    error_message: Option<String>,
}

#[derive(Debug)]
struct OpenOptions {
    system_prompt: bool,
    recall_mode: bool,
    caching_mechanism: bool,
}
#[derive(Debug)]
struct InitPromptUi {
    messages: MessageVector,
    current_message_idx: usize,
    current_message_content: String,
}
#[derive(Debug)]
struct CachingMechanismUi {
    options_open: bool,
    mech: CachingMechanism,
    limit: f32,
    long_term_enabled: bool,
    mech_replacement: Option<CachingMechanism>,
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

impl From<CachingMechanism> for CachingMechanismUi {
    fn from(value: CachingMechanism) -> Self {
        let mech = value;
        let limit = mech.limit() as f32;
        let long_term_enabled = match mech {
            CachingMechanism::SummarizeAtLimit { save_to_lt, .. } => save_to_lt,
            _ => false,
        };
        Self {
            options_open: false,
            mech,
            limit,
            long_term_enabled,
            mech_replacement: None,
        }
    }
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

impl CachingMechanismUi {
    fn mech_name(&self) -> String {
        String::from(match &self.mech {
            CachingMechanism::Forgetful => "Forgetful",
            CachingMechanism::SummarizeAtLimit { .. } => "SummarizeAtLimit",
        })
    }
    fn change_to_replacement(&mut self) {
        if let Some(new_mech) = self.mech_replacement.take() {
            let mech = new_mech;
            let limit = mech.limit() as f32;
            let long_term_enabled = mech.long_term_enabled();

            self.mech = mech;
            self.limit = limit;
            self.long_term_enabled = long_term_enabled;
        }
    }
    fn options_display(&mut self, ui: &mut egui::Ui) {
        ui.indent("CachingOptions", |ui| {
            if let None = self.mech_replacement {
                self.mech_replacement = Some(CachingMechanism::Forgetful);
            }
            ui.radio_value(
                &mut self.mech_replacement,
                Some(CachingMechanism::Forgetful),
                "Forgetful",
            );
            ui.radio_value(
                &mut self.mech_replacement,
                Some(CachingMechanism::default_summary_at_limit()),
                "SummarizeAtLimit",
            );
            if self.mech_replacement != Some(CachingMechanism::Forgetful) {
                let upper_bounds = 100.0;
                let lower_bounds = 10.0;
                ui.add(
                    egui::Slider::new(&mut self.limit, lower_bounds..=upper_bounds)
                        .text("Cache size limit"),
                );
                ui.checkbox(&mut self.long_term_enabled, "Save to LTM");
            }

            if ui.button("💾").clicked() {
                let mech = &self.mech_replacement;
                self.mech_replacement = match mech {
                    Some(CachingMechanism::Forgetful) => Some(CachingMechanism::Forgetful),
                    Some(CachingMechanism::SummarizeAtLimit { .. }) => {
                        Some(CachingMechanism::SummarizeAtLimit {
                            limit: self.limit as usize,
                            save_to_lt: self.long_term_enabled,
                        })
                    }
                    None => None,
                };
                self.change_to_replacement();
                self.options_open = false;
            }
        });
    }
    fn overview_display(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.colored_label(
                Color32::GOLD,
                format!("Caching limit: {}", &self.mech.limit()),
            );

            if self.long_term_enabled {
                ui.colored_label(
                    Color32::GOLD,
                    RichText::new("LTM").font(FontId::proportional(9.0)),
                );
            } else {
            }
        });

        if ui
            .selectable_label(self.options_open, "Change Mechanism")
            .clicked()
        {
            self.options_open = !self.options_open;
        }

        if self.options_open {
            self.options_display(ui);
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

    fn display_buttons(ui: &mut egui::Ui, prompt_ui_rc: &Rc<RefCell<InitPromptUi>>) {
        ui.horizontal(|ui| {
            let mut prompt_ui = prompt_ui_rc.borrow_mut();

            let empty_message = Message::new_standard(
                espionox::context::memory::MessageRole::System,
                &String::new(),
            );

            let last_button = Button::new("<").small();
            let next_button = Button::new(">").small();
            let has_last = &prompt_ui.has_last();
            let has_next = &prompt_ui.has_next();

            ui.add_enabled_ui(*has_last, |ui| {
                if ui.add(last_button).clicked() {
                    prompt_ui.change_idx(-1);
                }
            });

            if ui.small_button("+").clicked() {
                prompt_ui.messages.push(empty_message.clone());
            }

            ui.add_enabled_ui(*has_next, |ui| {
                if ui.add(next_button).clicked() {
                    prompt_ui.change_idx(1);
                }
            });

            ui.add_space(100.0);

            ui.label(format!(
                "{}/{}",
                prompt_ui.current_message_idx + 1,
                prompt_ui.messages.len()
            ));
        });
    }

    fn display_message_input(ui: &mut egui::Ui, prompt_ui_rc: &Rc<RefCell<InitPromptUi>>) {
        ui.vertical(|ui| {
            let mut prompt_ui = prompt_ui_rc.borrow_mut();

            let textedit = egui::TextEdit::multiline(&mut prompt_ui.current_message_content)
                .hint_text("Write your prompt here");

            ui.add(textedit);
        });
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
            open: OpenOptions::default(),
            init_prompt_ui,
            recall_mode: RecallMode::default(),
            caching_mechanism_ui: CachingMechanism::default().into(),
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
            open: OpenOptions::default(),
            init_prompt_ui,
            recall_mode: agent.memory.recall_mode().clone(),
            caching_mechanism_ui: agent.memory.caching_mechanism().clone().into(),
            long_term_memory: LongTermMemory::None,
            error_message: None,
        }
    }
    fn init_prompt_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            let prompt_ui_rc = Rc::clone(&self.init_prompt_ui);
            InitPromptUi::display_message_input(ui, &prompt_ui_rc);
            InitPromptUi::display_buttons(ui, &prompt_ui_rc);
        });
    }

    fn recall_mode(&mut self, ui: &mut egui::Ui) {
        let current_mode = &self.recall_mode;
        ui.colored_label(
            Color32::KHAKI,
            format!("Current recall mode: {:?}", current_mode),
        );
    }

    fn caching_mechanism(&mut self, ui: &mut egui::Ui) {
        self.caching_mechanism_ui.overview_display(ui);
    }

    pub fn display_form(&mut self, ui: &mut egui::Ui, frontend: &FrontendComms) {
        if ui
            .selectable_label(self.open.system_prompt, "System Prompt")
            .clicked()
        {
            self.open.system_prompt = !self.open.system_prompt;
        }
        if self.open.system_prompt {
            self.init_prompt_ui(ui);
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
            self.caching_mechanism(ui);
        }
    }
}
