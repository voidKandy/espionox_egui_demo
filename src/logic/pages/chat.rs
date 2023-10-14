// use super::modals::AgentInfoModal;
use crate::logic::comms::{BackendCommand, FrontendComms, FrontendRequest};
use espionox::context::memory::{Message, MessageRole, MessageVector, ToMessage};

use super::egui;

use eframe::{
    egui::{CentralPanel, SidePanel},
    emath::Align2,
    epaint::Color32,
};

#[derive(Debug)]
pub struct Chat {
    name: String,
    chat_buffer: MessageVector,
    current_exchange: CurrentExchange,
    processing_response: bool,
    error_message: Option<String>,
}

#[derive(Debug)]
pub struct ChatPage {
    current_chat_name: Option<String>,
    chats: Vec<Chat>,
    create_new_chat_modal_open: bool,
    // create_chat_modal: AgentInfoModal,
}

#[derive(Default, Debug, Clone)]
pub struct CurrentExchange {
    pub user_input: String,
    pub stream_buffer: Option<String>,
}
impl CurrentExchange {
    fn push_to_stream_buffer(&mut self, token: &str) {
        match &mut self.stream_buffer {
            Some(buffer) => {
                buffer.push_str(token);
            }
            None => {
                self.stream_buffer = Some(token.to_string());
            }
        }
    }
}
impl ChatPage {
    pub fn init() -> Self {
        // let current_chat_name = agent_names[0].to_owned();
        // let chats = Self::init_chats(agent_names.to_vec());
        let chats = vec![];
        let current_chat_name = None;
        Self {
            current_chat_name,
            chats,
            create_new_chat_modal_open: false,
            // create_chat_modal: AgentInfoModal::default(),
        }
    }

    fn listen_for_chat_updates(&mut self, frontend: &FrontendComms, ctx: &egui::Context) {
        if let Ok(response) = frontend.receiver.lock().unwrap().try_recv() {
            tracing::info!("Frontend got response: {:?}", response);
            match response {
                FrontendRequest::DoneStreaming { chat_name } => {
                    let chat = self
                        .get_chat_by_name(&chat_name)
                        .expect("Couldn't get chat with that name");
                    chat.processing_response = false;
                    if chat.current_exchange.stream_buffer.is_some() {
                        chat.chat_buffer.as_mut().push(
                            chat.current_exchange
                                .stream_buffer
                                .take()
                                .unwrap()
                                .to_message(MessageRole::Assistant),
                        );
                    }
                }
                FrontendRequest::StreamToken { token, chat_name } => {
                    let chat = self
                        .get_chat_by_name(&chat_name)
                        .expect("Couldn't get chat with that name");
                    chat.current_exchange.push_to_stream_buffer(&token);
                    tracing::info!(
                        "Updated buffer: {}",
                        chat.current_exchange.stream_buffer.clone().unwrap()
                    );
                    ctx.request_repaint();
                }
                FrontendRequest::NewChatThread(chat_name) => {
                    let new_chat = Chat::init(&chat_name);
                    if self.current_chat_name.is_none() {
                        self.current_chat_name = Some(chat_name);
                    }
                    self.chats.push(new_chat);
                }
            }
        }
    }

    fn get_chat_by_name(&mut self, name: &str) -> Option<&mut Chat> {
        self.chats.iter_mut().find(|ch| ch.name == name)
    }

    pub fn all_chat_names(&self) -> Vec<String> {
        self.chats.iter().map(|ch| ch.name.to_string()).collect()
    }

    pub fn display_current_chat(&mut self, frontend: &FrontendComms, outer_ui: &mut egui::Ui) {
        // if self.create_new_chat_modal_open {
        //     self.display_modal(outer_ui, frontend);
        // }

        self.listen_for_chat_updates(frontend, outer_ui.ctx());

        let chat_names = self.all_chat_names().clone();
        // let chats = &mut self.chats;

        SidePanel::new(egui::panel::Side::Left, "ChatsPanel")
            .resizable(false)
            .show(outer_ui.ctx(), |ui| {
                let add_button_value = match self.create_new_chat_modal_open {
                    true => "➖",
                    false => "➕",
                };
                if ui.button(add_button_value).clicked() {
                    self.create_new_chat_modal_open = !self.create_new_chat_modal_open;
                }
                for (i, name) in chat_names.iter().enumerate() {
                    let is_selected = Some(name.to_string()) == self.current_chat_name;
                    ui.horizontal(|ui| {
                        if ui.radio(is_selected, name.to_string()).clicked() {
                            let new_chat_name = name.to_string();
                            self.current_chat_name = Some(new_chat_name);
                        }
                        match i {
                            0 => {}
                            _ => {
                                if ui.small_button("❌").clicked() {
                                    let chat_to_remove_name = name.to_string();
                                    let remove_command = BackendCommand::RemoveChatThread {
                                        name: chat_to_remove_name.to_owned(),
                                    };
                                    frontend.sender.try_send(remove_command).unwrap();
                                    if Some(chat_to_remove_name) == self.current_chat_name {
                                        self.current_chat_name = Some(chat_names[0].to_owned())
                                    }
                                    self.chats.retain(|ch| &ch.name != name)
                                }
                            }
                        }
                    });
                }
            });
        let current_chat_name = &self.current_chat_name.to_owned().unwrap();
        let chat = self.get_chat_by_name(current_chat_name).unwrap();
        chat.display(frontend, outer_ui);
    }
}

impl Chat {
    pub fn init(name: &str) -> Self {
        Self {
            name: name.to_string(),
            processing_response: false,
            chat_buffer: MessageVector::init(),
            current_exchange: CurrentExchange::default(),
            error_message: None,
        }
    }

    pub fn display(&mut self, frontend: &FrontendComms, outer_ui: &mut egui::Ui) {
        let mut scroll_to_bottom = false;
        let error_message = &mut self.error_message.clone();

        egui::Window::new("")
            .anchor(Align2::RIGHT_BOTTOM, [-5.0, -5.0])
            .auto_sized()
            .movable(false)
            .title_bar(false)
            .show(outer_ui.ctx(), |ui| {
                let user_input_box =
                    egui::TextEdit::multiline(&mut self.current_exchange.user_input)
                        .desired_rows(1)
                        .lock_focus(true)
                        .frame(false)
                        .hint_text("Send a message")
                        .vertical_align(eframe::emath::Align::BOTTOM);

                if error_message.is_some() {
                    ui.colored_label(Color32::RED, error_message.as_ref().unwrap());
                }

                let user_input_handle = ui.add(user_input_box);

                let shift_enter_pressed = user_input_handle.has_focus()
                    && ui.input(|i| i.modifiers.shift_only() && i.key_pressed(egui::Key::Enter));
                let submit_button_pressed = ui.button("Enter").clicked();
                let enter_pressed_with_content = user_input_handle.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.current_exchange.user_input.trim().is_empty();

                if shift_enter_pressed {
                    // Do nothing
                } else if enter_pressed_with_content || submit_button_pressed {
                    match self.processing_response {
                        true => {
                            *error_message =
                                Some("Please wait until current response is processed".to_string());
                        }
                        false => {
                            scroll_to_bottom = true;
                            self.chat_buffer.as_mut().push(
                                self.current_exchange
                                    .user_input
                                    .as_str()
                                    .to_message(MessageRole::User),
                            );
                            self.send_last_user_message_to_backend(frontend, outer_ui.ctx());
                            self.processing_response = true;
                        }
                    }
                }
            });

        CentralPanel::default().show(outer_ui.ctx(), |ui| {
            let chat_width = ui.available_size().x * 0.95;
            let chat_height = ui.available_size().y * 0.95;
            let chat_scroll_area = egui::ScrollArea::vertical()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .auto_shrink([false; 2])
                .max_height(chat_height)
                .max_width(chat_width)
                .stick_to_right(true);

            chat_scroll_area.show(ui, |ui| {
                let buffer = &mut self.chat_buffer.as_ref();
                for message in buffer.into_iter() {
                    let color = match message.role() {
                        MessageRole::User => egui::Color32::YELLOW,
                        MessageRole::Assistant => egui::Color32::GREEN,
                        _ => egui::Color32::DARK_RED,
                    };
                    ui.colored_label(color, message.content().unwrap());
                }

                if let Some(current_stream_buffer) = &mut self.current_exchange.stream_buffer {
                    let model_output = egui::TextEdit::multiline(current_stream_buffer)
                        .text_color(egui::Color32::GREEN)
                        .frame(false)
                        .interactive(false);
                    ui.add_sized([chat_width, chat_height], model_output);
                }
                ui.ctx().request_repaint();

                if scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
                ui.set_max_height(chat_height);
            });
        });
    }

    fn send_last_user_message_to_backend(&mut self, frontend: &FrontendComms, ctx: &egui::Context) {
        ctx.request_repaint();
        let backend_command = BackendCommand::StreamedCompletion {
            agent_name: self.name.to_owned(),
            prompt: self.current_exchange.user_input.to_owned(),
        };
        self.current_exchange.user_input.clear();

        frontend
            .sender
            .try_send(backend_command)
            .expect("Failed to send user input to backend");
    }
}
