use eframe::egui::{self, Button, Layout};
use espionox::{
    context::memory::{Message, MessageRole, MessageVector},
    persistance::prompts::{get_prompts_from_file, Prompt},
};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct InitPromptUi {
    loaded_prompts: Option<Vec<Prompt>>,
    show_loaded_prompts: bool,
    messages: MessageVector,
    current_message_idx: usize,
    current_message_content: String,
    current_message_role: MessageRole,
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
        let current_message_role = value.as_ref()[current_message_idx].role();
        let loaded_prompts = match get_prompts_from_file() {
            Ok(p) => Some(p),
            Err(_) => None,
        };
        Self {
            loaded_prompts,
            show_loaded_prompts: false,
            messages: value,
            current_message_idx,
            current_message_content,
            current_message_role,
        }
    }
}

impl InitPromptUi {
    pub fn init_prompt(&self) -> &MessageVector {
        &self.messages
    }

    fn save_current(&mut self) {
        let current = Message::new_standard(
            self.current_message_role.to_owned(),
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
        self.current_message_role = self.messages.as_ref()[self.current_message_idx].role();
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

    fn mutate_with_prompt(&mut self, prompt: Prompt) {
        let messages: MessageVector = prompt.messages.into();
        *self = Self::from(messages);
    }

    pub fn display_saved_prompt_options(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(Layout::top_down(eframe::emath::Align::Min), |ui| {
            if let Some(vec) = self.loaded_prompts.take() {
                for p in vec.iter() {
                    if ui.button(&p.name).clicked() {
                        self.mutate_with_prompt(p.clone());
                        self.show_loaded_prompts = false;
                        break;
                    }
                }
                self.loaded_prompts = Some(vec);
            }
        });
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

            if ui.small_button("Load").clicked() {
                prompt_ui.show_loaded_prompts = !prompt_ui.show_loaded_prompts;
            }
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
        let mut prompt_ui = prompt_ui_rc.borrow_mut();
        // ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let textedit = egui::TextEdit::multiline(&mut prompt_ui.current_message_content)
                .hint_text("Right click for more options");

            ui.add(textedit).context_menu(|ui| {
                ui.vertical(|ui| {
                    ui.label("Role");
                    if ui
                        .radio_value(
                            &mut prompt_ui.current_message_role,
                            MessageRole::System,
                            "Sys",
                        )
                        .clicked()
                    {
                        ui.close_menu()
                    };
                    if ui
                        .radio_value(
                            &mut prompt_ui.current_message_role,
                            MessageRole::User,
                            "User",
                        )
                        .clicked()
                    {
                        ui.close_menu()
                    };
                    if ui
                        .radio_value(
                            &mut prompt_ui.current_message_role,
                            MessageRole::Assistant,
                            "Ai",
                        )
                        .clicked()
                    {
                        ui.close_menu()
                    };
                });
            });

            if prompt_ui.show_loaded_prompts {
                prompt_ui.display_saved_prompt_options(ui);
            }
        });
    }

    pub fn overview_display(rc_ref_self: Rc<RefCell<Self>>, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            let prompt_ui_rc = Rc::clone(&rc_ref_self);
            InitPromptUi::display_message_input(ui, &prompt_ui_rc);
            InitPromptUi::display_buttons(ui, &prompt_ui_rc);
        });
    }
}
