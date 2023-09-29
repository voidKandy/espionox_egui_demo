use crate::logic::comms::{BackendCommand, FrontendComms, FrontendRequest};

use super::egui;

use eframe::{
    egui::{CentralPanel, SidePanel, TopBottomPanel},
    emath::Align2,
    epaint::Stroke,
};
use espionox::context::{Message, MessageVector};

/// Create a way to save multiple chats... Will need to tell backend which agent to use
#[derive(Debug)]
pub struct Chat {
    name: String,
    chat_buffer: MessageVector,
    current_exchange: CurrentExchange,
}

#[derive(Debug)]
pub struct ChatPage {
    // chats: Vec<Chat>,
    chat_buffer: MessageVector,
    current_exchange: CurrentExchange,
}

#[derive(Default, Debug, Clone)]
pub struct CurrentExchange {
    pub user_input: String,
    pub stream_buffer: Option<String>,
}

impl CurrentExchange {}

impl ChatPage {
    pub fn new() -> Self {
        Self {
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
                    scroll_to_bottom = true;
                    self.chat_buffer.as_mut().push(Message::new_standard(
                        "user",
                        self.current_exchange.user_input.as_str(),
                    ));
                    self.send_last_user_message_to_backend(frontend, outer_ui.ctx(), true);
                }
            });

        SidePanel::new(egui::panel::Side::Left, "ChatsPanel")
            .resizable(false)
            .show(outer_ui.ctx(), |ui| {
                ui.radio(true, "Some chat");
                ui.radio(false, "Another one");
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
            match response {
                FrontendRequest::DoneStreaming => {
                    if let Some(s) = &self.current_exchange.stream_buffer {
                        println!("We pushed: {}", s);
                        self.chat_buffer.as_mut().push(Message::new_standard(
                            "assistant",
                            &self.current_exchange.stream_buffer.take().unwrap(),
                        ));
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

    fn send_last_user_message_to_backend(
        &mut self,
        frontend: &FrontendComms,
        ctx: &egui::Context,
        stream: bool,
    ) {
        ctx.request_repaint();
        let backend_command = match stream {
            false => BackendCommand::SingleCompletion(self.current_exchange.user_input.to_owned()),
            true => BackendCommand::StreamedCompletion(self.current_exchange.user_input.to_owned()),
        };
        self.current_exchange.user_input.clear();

        frontend
            .sender
            .try_send(backend_command)
            .expect("Failed to send user input to backend");
    }
}
