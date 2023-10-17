use eframe::{
    egui::{self, Button, RichText},
    epaint::{Color32, FontId},
};
use espionox::context::memory::{CachingMechanism, Message, MessageRole, MessageVector};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub(super) struct InitPromptUi {
    messages: MessageVector,
    current_message_idx: usize,
    current_message_content: String,
}
#[derive(Debug)]
pub(super) struct CachingMechanismUi {
    options_open: bool,
    mech: CachingMechanism,
    limit: f32,
    long_term_enabled: bool,
    mech_replacement: Option<CachingMechanism>,
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
    pub fn mech_name(&self) -> String {
        String::from(match &self.mech {
            CachingMechanism::Forgetful => "Forgetful",
            CachingMechanism::SummarizeAtLimit { .. } => "SummarizeAtLimit",
        })
    }

    pub fn caching_mechanism(&self) -> &CachingMechanism {
        &self.mech
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

    pub fn overview_display(&mut self, ui: &mut egui::Ui) {
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

    pub fn init_prompt(&self) -> &MessageVector {
        &self.messages
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

    pub fn display_buttons(ui: &mut egui::Ui, prompt_ui_rc: &Rc<RefCell<InitPromptUi>>) {
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

    pub fn display_message_input(ui: &mut egui::Ui, prompt_ui_rc: &Rc<RefCell<InitPromptUi>>) {
        ui.vertical(|ui| {
            let mut prompt_ui = prompt_ui_rc.borrow_mut();

            let textedit = egui::TextEdit::multiline(&mut prompt_ui.current_message_content)
                .hint_text("Write your prompt here");

            ui.add(textedit);
        });
    }
}
