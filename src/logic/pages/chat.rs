use std::collections::HashMap;

use crate::logic::comms::{BackendCommand, FrontendComms, FrontendRequest};

use super::egui;

use eframe::{
    egui::{CentralPanel, SidePanel, TopBottomPanel},
    emath::Align2,
};
use espionox::context::{Message, MessageVector};

/// Create a way to save multiple chats... Will need to tell backend which agent to use
#[derive(Debug)]
pub struct Chat {
    name: String,
    chat_buffer: MessageVector,
    current_exchange: CurrentExchange,
    processing_response: bool,
}

#[derive(Debug)]
pub struct ChatPage {
    current_chat: String,
    chats: HashMap<String, Chat>,
    // processing_response: bool,
    // chat_buffer: MessageVector,
    // current_exchange: CurrentExchange,
}

#[derive(Default, Debug, Clone)]
pub struct CurrentExchange {
    pub user_input: String,
    pub stream_buffer: Option<String>,
}

impl ChatPage {
    pub fn init(agent_names: Vec<String>) -> Self {
        let current_chat = agent_names[0].to_owned();
        let chats = Self::init_chats(agent_names);
        Self {
            current_chat,
            chats,
        }
    }
    fn init_chats(names: Vec<String>) -> HashMap<String, Chat> {
        let mut map = HashMap::new();
        names.into_iter().for_each(|n| {
            let chat = Chat::new(&n);
            map.insert(n, chat);
        });
        map
    }
    fn chat_names(&self) -> Vec<String> {
        self.chats.keys().cloned().collect()
    }

    pub fn display_current_chat(&mut self, frontend: &FrontendComms, outer_ui: &mut egui::Ui) {
        SidePanel::new(egui::panel::Side::Left, "ChatsPanel")
            .resizable(false)
            .show(outer_ui.ctx(), |ui| {
                for name in self.chat_names().iter() {
                    if ui.radio(name == &self.current_chat, name).clicked() {
                        self.current_chat = name.to_string();
                    }
                }
            });

        let chat = self.chats.get_mut(&self.current_chat).unwrap();
        chat.display(frontend, outer_ui);
    }
}

impl Chat {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            processing_response: false,
            chat_buffer: MessageVector::init(),
            current_exchange: CurrentExchange::default(),
        }
    }

    pub fn display(&mut self, frontend: &FrontendComms, outer_ui: &mut egui::Ui) {
        let mut scroll_to_bottom = false;

        egui::Window::new("My Window")
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
                let user_input_handle = ui.add(user_input_box);

                let shift_enter_pressed = user_input_handle.has_focus()
                    && ui.input(|i| i.modifiers.shift_only() && i.key_pressed(egui::Key::Enter));
                let enter_pressed_with_content = user_input_handle.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.current_exchange.user_input.trim().is_empty();

                if shift_enter_pressed {
                    // self.current_exchange.user_input =
                    //     format!("\n{}", self.current_exchange.user_input);
                } else if enter_pressed_with_content {
                    match self.processing_response {
                        true => {
                            // SOmehow handle users trying to overwhelm the model
                        }
                        false => {
                            scroll_to_bottom = true;
                            self.chat_buffer.as_mut().push(Message::new_standard(
                                "user",
                                self.current_exchange.user_input.as_str(),
                            ));
                            self.send_last_user_message_to_backend(frontend, outer_ui.ctx());
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
                    let color = match message.role().as_str() {
                        "user" => egui::Color32::YELLOW,
                        "assistant" => egui::Color32::GREEN,
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

                self.update_stream_buffer_with_backend_response(frontend, ui.ctx());

                if scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
                ui.set_max_height(chat_height);
            });
        });
    }

    fn update_stream_buffer_with_backend_response(
        &mut self,
        frontend: &FrontendComms,
        ctx: &egui::Context,
    ) {
        if let Ok(response) = frontend.receiver.lock().unwrap().try_recv() {
            self.processing_response = true;
            match response {
                FrontendRequest::DoneStreaming => {
                    if let Some(_) = &self.current_exchange.stream_buffer {
                        self.chat_buffer.as_mut().push(Message::new_standard(
                            "assistant",
                            &self.current_exchange.stream_buffer.take().unwrap(),
                        ));
                        self.processing_response = false;
                    }
                }
                _ => {
                    let res: String = response.into();
                    match &self.current_exchange.stream_buffer {
                        Some(buffer) => {
                            self.current_exchange.stream_buffer = Some(format!("{}{}", buffer, res))
                        }
                        None => self.current_exchange.stream_buffer = Some(res),
                    }
                    ctx.request_repaint();
                }
            }
        }
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
