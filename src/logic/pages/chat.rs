use crate::logic::comms::{BackendCommand, FrontendComms, FrontendRequest};

use super::egui;

use eframe::{
    egui::{CentralPanel, SidePanel},
    emath::Align2,
};
use espionox::{
    agent::AgentSettings,
    context::{
        long_term::LongTermMemory,
        short_term::{MemoryCache, ShortTermMemory},
        Message, MessageVector,
    },
};

#[derive(Debug)]
pub struct Chat {
    name: String,
    chat_buffer: MessageVector,
    current_exchange: CurrentExchange,
    processing_response: bool,
}

#[derive(Debug)]
pub struct ChatPage {
    current_chat_name: String,
    chats: Vec<Chat>,
    create_new_chat_modal_open: bool,
    create_chat_modal: CreateNewChatModal,
}

#[derive(Default, Debug, Clone)]
pub struct CurrentExchange {
    pub user_input: String,
    pub stream_buffer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateNewChatModal {
    chat_name: String,
    init_prompt: String,
    selected_stm: ShortTermMemory,
    selected_ltm: LongTermMemory,
}

impl Default for CreateNewChatModal {
    fn default() -> Self {
        Self {
            chat_name: String::new(),
            init_prompt: String::new(),
            selected_stm: ShortTermMemory::default(),
            selected_ltm: LongTermMemory::default(),
        }
    }
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

impl CreateNewChatModal {
    fn settings(&self) -> AgentSettings {
        AgentSettings::new()
            .short_term(self.selected_stm.clone())
            .init_prompt(MessageVector::init_with_system_prompt(&self.init_prompt))
            // .long_term(self.selected_ltm)
            .finish()
    }
    fn display_modal(&mut self, ui: &mut egui::Ui, frontend: &FrontendComms, open: &mut bool) {
        egui::Window::new("Create New Chat")
            .open(open)
            .show(ui.ctx(), |ui| {
                // let mut stm = &self.selected_stm;
                ui.add(
                    egui::TextEdit::singleline(&mut self.chat_name)
                        .hint_text("Put a chat name here"),
                );
                ui.label("Short Term Memory");
                ui.radio_value(&mut self.selected_stm, ShortTermMemory::Forget, "No STM");
                ui.radio_value(
                    &mut self.selected_stm,
                    ShortTermMemory::Cache(MemoryCache::default()),
                    "Cached Memory",
                );
                ui.label("Long Term Memory");
                ui.radio_value(&mut self.selected_ltm, LongTermMemory::None, "No LTM");
                // let mut init_prompt = String::new();
                let init_prompt_te = egui::TextEdit::multiline(&mut self.init_prompt)
                    .hint_text("Put your desired system prompt here");
                let _ = ui.add(init_prompt_te);
                if ui.button("Create New Chat").clicked() {
                    if self.chat_name.is_empty() {
                        ui.label("Please fill out the name field");
                    } else {
                        let name = &self.chat_name;
                        let name = name.to_string();
                        let settings = self.settings();
                        let create_command = BackendCommand::NewChatThread { name, settings };
                        frontend
                            .sender
                            .try_send(create_command)
                            .expect("Failed to send chat creation command")
                    }
                    // self.create_new_chat_modal_open = false;
                }
            });
    }
}

impl ChatPage {
    pub fn init(agent_names: &Vec<String>) -> Self {
        let current_chat_name = agent_names[0].to_owned();
        let chats = Self::init_chats(agent_names.to_vec());
        Self {
            current_chat_name,
            chats,
            create_new_chat_modal_open: false,
            create_chat_modal: CreateNewChatModal::default(),
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
                    if chat.current_exchange.stream_buffer.is_some() {
                        chat.chat_buffer.as_mut().push(Message::new_standard(
                            "assistant",
                            &chat.current_exchange.stream_buffer.take().unwrap(),
                        ));
                        chat.processing_response = false;
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
                FrontendRequest::NewChatThread(name) => {
                    let new_chat = Chat::new(&name);
                    self.chats.push(new_chat);
                }
            }
        }
    }

    fn init_chats(names: Vec<String>) -> Vec<Chat> {
        let mut vec = Vec::new();
        names.into_iter().for_each(|n| {
            let chat = Chat::new(&n);
            vec.push(chat);
        });
        vec
    }

    fn get_chat_by_name(&mut self, name: &str) -> Option<&mut Chat> {
        self.chats.iter_mut().find(|ch| ch.name == name)
    }

    pub fn display_current_chat(&mut self, frontend: &FrontendComms, outer_ui: &mut egui::Ui) {
        if self.create_new_chat_modal_open {
            self.create_chat_modal.display_modal(
                outer_ui,
                frontend,
                &mut self.create_new_chat_modal_open,
            );
        }

        self.listen_for_chat_updates(frontend, outer_ui.ctx());

        SidePanel::new(egui::panel::Side::Left, "ChatsPanel")
            .resizable(false)
            .show(outer_ui.ctx(), |ui| {
                if ui.small_button("➕").clicked() {
                    self.create_new_chat_modal_open = true;
                }
                for name in self.chats.iter().map(|ch| &ch.name) {
                    let is_selected = *name == self.current_chat_name;
                    ui.horizontal(|ui| {
                        if ui.radio(is_selected, name.to_string()).clicked() {
                            let new_chat_name = name.to_string();
                            self.current_chat_name = new_chat_name;
                        }
                        if ui.small_button("❌").clicked() {
                            egui::Window::new("SettingsWindow").show(ui.ctx(), |ui| {
                                ui.label("Hello World!");
                            });
                        }
                    });
                }
            });
        let chat = self
            .get_chat_by_name(&self.current_chat_name.to_owned())
            .unwrap();
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
                    // Do nothing
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
                            // self.thread_sender
                            //     .try_send(self.current_exchange.user_input)
                            //     .expect("Failed to send to backend chat thread");
                            // self.current_exchange.user_input.clear();
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
                // self.update_stream_buffer_with_backend_response(frontend, ui.ctx());
                ui.ctx().request_repaint();

                if scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
                ui.set_max_height(chat_height);
            });
        });
    }

    // fn update_stream_buffer_with_backend_response(
    //     &mut self,
    //     frontend: &FrontendComms,
    //     ctx: &egui::Context,
    // ) {
    //     if let Ok(response) = frontend.receiver.lock().unwrap().try_recv() {
    //         tracing::info!("Frontend got response: {:?}", response);
    //         match response {
    //             FrontendRequest::DoneStreaming => {
    //                 if self.current_exchange.stream_buffer.is_some() {
    //                     self.chat_buffer.as_mut().push(Message::new_standard(
    //                         "assistant",
    //                         &self.current_exchange.stream_buffer.take().unwrap(),
    //                     ));
    //                     self.processing_response = false;
    //                 }
    //             }
    //             FrontendRequest::StreamToken { token, chat_name } => {
    //                 self.current_exchange.push_to_stream_buffer(&token);
    //                 tracing::info!(
    //                     "Updated buffer: {}",
    //                     self.current_exchange.stream_buffer.clone().unwrap()
    //                 );
    //                 ctx.request_repaint();
    //             }
    //             _ => {}
    //         }
    //     }
    // }

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
