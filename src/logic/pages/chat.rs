use crate::logic::comms::{BackendCommand, FrontendComms};

use super::egui;

use eframe::egui::{CentralPanel, SidePanel, TopBottomPanel};
use espionox::context::{Message, MessageVector};

#[derive(Debug)]
pub struct ChatPage {
    user_input: String,
    chat_buffer: MessageVector,
    // comms: FrontendComms,
}

impl ChatPage {
    pub fn new() -> Self {
        Self {
            user_input: String::new(),
            chat_buffer: MessageVector::init(),
            // comms,
        }
    }
}
impl ChatPage {
    pub fn display(&mut self, frontend: &FrontendComms, outer_ui: &mut egui::Ui) {
        let mut scroll_to_bottom = false;

        // SidePanel::new(egui::panel::Side::Right, "OptionsPanel")
        //     .resizable(false)
        //     .show_separator_line(false)
        //     .show(ui.ctx(), |ui| {
        //         ui.menu_button("Templates", |ui| {
        //             if ui.small_button("template").clicked() {
        //                 println!();
        //             }
        //         });
        //     });

        // egui::Area::new("UserInput")
        //     .anchor(egui::Align2::LEFT_BOTTOM, [0.0, 0.0])
        //     .pivot(egui::Align2::CENTER_CENTER)
        TopBottomPanel::bottom("user_input_panel")
            .show_separator_line(false)
            .resizable(false)
            .show(outer_ui.ctx(), |ui| {
                ui.horizontal_centered(|ui| {
                    let user_input_box = egui::TextEdit::multiline(&mut self.user_input)
                        .desired_rows(1)
                        .lock_focus(true)
                        .clip_text(true)
                        .horizontal_align(eframe::emath::Align::LEFT)
                        .vertical_align(eframe::emath::Align::TOP);
                    let user_input_width = ui.available_size().x * 0.9;
                    let user_input_height = ui.available_size().y * 0.15;
                    let user_input_box_size: (f32, f32) = (user_input_width, user_input_height);

                    let user_input_handle = ui.add_sized(user_input_box_size, user_input_box);

                    if ui
                        .button("Templates")
                        .on_hover_text("Choose from your templates")
                        .clicked()
                    {
                        todo!();
                    }

                    let shift_enter_pressed = user_input_handle.has_focus()
                        && ui
                            .input(|i| i.modifiers.shift_only() && i.key_pressed(egui::Key::Enter));
                    let enter_pressed_with_content = user_input_handle.has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && !self.user_input.trim().is_empty();

                    if shift_enter_pressed {
                        self.user_input = format!("{}\n", self.user_input);
                    } else if enter_pressed_with_content {
                        scroll_to_bottom = true;

                        self.chat_buffer
                            .as_mut()
                            .push(Message::new_standard("user", self.user_input.as_str()));
                        self.send_last_user_message_to_backend(frontend, outer_ui.ctx(), true);
                    }
                });
            });

        CentralPanel::default().show(outer_ui.ctx(), |ui| {
            let chat_width = ui.available_size().x;
            let chat_height = ui.available_size().y * 0.95;
            let chat_scroll_area = egui::ScrollArea::vertical()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .auto_shrink([false; 2])
                .max_height(chat_height)
                .stick_to_right(true);

            chat_scroll_area.show(ui, |ui| {
                let buffer = &mut self.chat_buffer.to_string();
                ui.ctx().request_repaint();

                self.update_chat_buffer_with_backend_response(frontend, ui.ctx());

                let chat_display = egui::TextEdit::multiline(buffer)
                    .frame(false)
                    .interactive(false)
                    .horizontal_align(eframe::emath::Align::Min);
                let chat_size: (f32, f32) = (chat_width, chat_height);
                ui.add_sized(chat_size, chat_display);

                if scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
                ui.set_max_height(chat_height);
            });
        });
    }

    fn send_last_user_message_to_backend(
        &mut self,
        frontend: &FrontendComms,
        ctx: &egui::Context,
        stream: bool,
    ) {
        ctx.request_repaint();
        let backend_command = match stream {
            false => BackendCommand::SingleCompletion(self.user_input.to_owned()),
            true => BackendCommand::StreamedCompletion(self.user_input.to_owned()),
        };
        self.user_input.clear();

        frontend
            .sender
            .try_send(backend_command)
            .expect("Failed to send user input to backend");
    }

    fn update_chat_buffer_with_backend_response(
        &mut self,
        frontend: &FrontendComms,
        ctx: &egui::Context,
    ) {
        if let Ok(response) = frontend.receiver.lock().unwrap().try_recv() {
            let res: String = response.into();
            self.chat_buffer
                .as_mut()
                .push(Message::new_standard("assistant", &res));
            ctx.request_repaint();
        }
    }
}
