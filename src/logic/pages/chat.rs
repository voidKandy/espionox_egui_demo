use super::modals::AgentInfoModal;
use crate::logic::comms::{BackendCommand, FrontendComms, FrontendRequest};
use espionox::context::memory::{MessageRole, MessageVector, ToMessage};

use eframe::egui;

use eframe::{
    egui::{CentralPanel, Id, RichText, Separator, SidePanel},
    emath::Align2,
    epaint::{Color32, FontId},
};
use espionox::core::{Directory, File};

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
    agent_info_modal: AgentInfoModal,
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
            agent_info_modal: AgentInfoModal::new_empty(),
        }
    }

    // MAKE THIS TAKE  TRAIT WHICH HAS  METHODS:
    // * Display_form()
    // * window_name() -> String
    pub fn display_new_chat_modal(&mut self, ui: &mut egui::Ui, frontend: &FrontendComms) {
        // let existing_names = self.all_chat_names();
        let modal = &mut self.agent_info_modal;
        let x = ui.available_width() / 2.0;
        let y = ui.available_height() / 2.0;
        egui::Window::new("New Chat")
            .title_bar(false)
            .collapsible(false)
            .fixed_size((x, y))
            .anchor(Align2::CENTER_CENTER, [-10.0, 0.0])
            .show(ui.ctx(), |ui| {
                let ui_width = ui.max_rect().width() / 2.0;
                ui.set_max_width(ui_width);
                ui.vertical_centered(|ui| {
                    ui.colored_label(
                        Color32::LIGHT_BLUE,
                        RichText::new("New Chat")
                            .font(FontId::proportional(18.0))
                            .strong(),
                    );
                    if ui.small_button("create").clicked() {
                        if let Ok(new_thread_command) = modal.try_into() {
                            frontend.sender.try_send(new_thread_command).unwrap();
                            *modal = AgentInfoModal::new_empty();
                            self.create_new_chat_modal_open = false;
                        }
                    };
                });
                ui.add(Separator::default().horizontal());
                if let Some(err_mess) = &modal.error_message {
                    ui.colored_label(Color32::RED, err_mess);
                }
                modal.display_agent_form(ui);
            });
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
        let open_modal = self.create_new_chat_modal_open;
        if open_modal {
            self.display_new_chat_modal(outer_ui, frontend);
        }

        self.listen_for_chat_updates(frontend, outer_ui.ctx());

        let chat_names = self.all_chat_names().clone();

        SidePanel::new(egui::panel::Side::Left, "ChatsPanel")
            .resizable(false)
            .show(outer_ui.ctx(), |ui| {
                let add_button_value = match self.create_new_chat_modal_open {
                    true => "➖",
                    false => "➕",
                };

                for name in chat_names.iter() {
                    let is_selected = Some(name.to_string()) == self.current_chat_name;
                    ui.horizontal(|ui| {
                        let chat_selector =
                            ui.radio(is_selected, name.to_string()).context_menu(|ui| {
                                ui.set_width(1.0);
                                if chat_names.len() > 1 {
                                    if ui.button("❌").clicked() {
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
                            });

                        if chat_selector.clicked() {
                            let new_chat_name = name.to_string();
                            self.current_chat_name = Some(new_chat_name);
                        }
                        // FUTURE FEATURE: CHANGING AGENTS
                        // if ui.small_button("≡").clicked() {
                        //     // Method for returning agent reference from backend command??
                        //     let get_agent_reference_command = BackendCommand::AgentRef { name };
                        //     frontend
                        //         .sender
                        //         .try_send(get_agent_reference_command)
                        //         .unwrap();
                        //     thread::sleep(Duration::from_millis(200));
                        //     if let Some(response) = frontend.recv() {
                        //         let agent = response.into();
                        //         self.agent_info_modal = AgentInfoModal::from(agent, name);
                        //     }
                        // }
                    });
                }

                ui.add_space(10.0);
                if ui.button(add_button_value).clicked() {
                    self.create_new_chat_modal_open = !self.create_new_chat_modal_open;
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

    fn handle_main_chat_interface(&mut self, ui: &mut egui::Ui) {
        let buffer = &mut self.chat_buffer.as_ref();
        let chat_width = ui.available_width();
        let chat_height = ui.available_height();
        let font_size = 16.0;

        for message in buffer.into_iter() {
            let content = message.content().unwrap_or(String::new());
            let content = match message.role() {
                MessageRole::User => format!("👤 {}", content),
                _ => format!("💻 {}", content),
            };
            let code_chunk_split = content.split("```");
            for (i, mut string) in code_chunk_split.into_iter().enumerate() {
                let color = match message.role() {
                    MessageRole::User => Color32::from_rgb(255, 223, 223),
                    _ => Color32::from_rgb(210, 220, 255),
                };
                let font = match i % 2 {
                    0 => FontId::proportional(font_size),
                    _ => FontId::monospace(font_size),
                };

                ui.horizontal(|ui| {
                    // ui.set_width(chat_width * 0.8);
                    // let label = Label::new(richtext.size(font_size)).wrap(true);
                    // ui.add(label);
                    ui.add(
                        egui::TextEdit::multiline(&mut string)
                            .desired_width(chat_width)
                            .text_color(color)
                            .frame(false)
                            .font(font)
                            .margin([2.0, 1.0].into()),
                    );
                });
            }
        }

        if let Some(current_stream_buffer) = &mut self.current_exchange.stream_buffer {
            let model_output = egui::TextEdit::multiline(current_stream_buffer)
                .font(FontId::proportional(font_size))
                .frame(false)
                .interactive(false);

            // ui.add(model_output);
            ui.add_sized([chat_width, chat_height], model_output);
        }
        ui.ctx().request_repaint();
    }

    pub fn display(&mut self, frontend: &FrontendComms, outer_ui: &mut egui::Ui) {
        let mut scroll_to_bottom = false;
        let error_message = &mut self.error_message.clone();

        egui::Window::new("")
            .id(Id::new("user_input_window"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -10.0])
            .auto_sized()
            .movable(false)
            .title_bar(false)
            .show(outer_ui.ctx(), |ui| {
                ui.horizontal(|ui| {
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

                    let enter_button = egui::Button::new("⮊");
                    let enter_button_handle = match self.processing_response {
                        true => ui.spinner(),
                        false => ui
                            .add(enter_button)
                            .on_hover_text("Right click for more options")
                            .context_menu(|ui| {
                                if ui.button("Add File").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter("plaintext", &["txt", "md"])
                                        .add_filter(
                                            "code",
                                            &["rs", "toml", "yaml", "py", "js", "ts", "c", "json"],
                                        )
                                        .set_directory("/")
                                        .pick_file()
                                    {
                                        let file = File::from(path);
                                        let message = file.to_message(MessageRole::User);
                                        frontend
                                            .sender
                                            .try_send(BackendCommand::PushToAgentMemory {
                                                agent_name: self.name.to_string(),
                                                message,
                                            })
                                            .unwrap();
                                    }
                                }

                                if ui.button("Add Directory").clicked() {
                                    if let Some(path) =
                                        rfd::FileDialog::new().set_directory("/").pick_folder()
                                    {
                                        let directory = Directory::from(path);
                                        let message = directory.to_message(MessageRole::User);

                                        frontend
                                            .sender
                                            .try_send(BackendCommand::PushToAgentMemory {
                                                agent_name: self.name.to_string(),
                                                message,
                                            })
                                            .unwrap();
                                    }
                                }
                            }),
                    };

                    let shift_enter_pressed = user_input_handle.has_focus()
                        && ui
                            .input(|i| i.modifiers.shift_only() && i.key_pressed(egui::Key::Enter));

                    let submit_button_pressed = enter_button_handle.clicked();

                    let enter_pressed_with_content = user_input_handle.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && !self.current_exchange.user_input.trim().is_empty();
                    if shift_enter_pressed {
                        // Do nothing
                    } else if enter_pressed_with_content || submit_button_pressed {
                        match self.processing_response {
                            true => {
                                *error_message = Some(
                                    "Please wait until current response is processed".to_string(),
                                );
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
                self.handle_main_chat_interface(ui);
            });

            if scroll_to_bottom {
                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
            }
            ui.set_max_height(chat_height);
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
