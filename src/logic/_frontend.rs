use super::{comms::FrontendComms, AppBackend, BackendCommand};
use eframe::{egui, emath::Align2};
use espionox::context::MessageVector;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct AppFrontend {
    pub sender: Arc<mpsc::Sender<BackendCommand>>,
    pub receiver: Arc<Mutex<mpsc::Receiver<FrontendRequest>>>,
}

#[derive(Debug, Clone)]
pub enum FrontendRequest {
    Message(String),
    Done,
}
unsafe impl Send for FrontendRequest {}
unsafe impl Sync for FrontendRequest {}

impl From<String> for FrontendRequest {
    fn from(str: String) -> Self {
        Self::Message(str)
    }
}

impl Into<String> for FrontendRequest {
    fn into(self) -> String {
        match self {
            Self::Message(string) => string,
            Self::Done => "Done".to_string(),
        }
    }
}

impl AppFrontend {
    const USER_COLOR: egui::Color32 = egui::Color32::from_rgb(155, 240, 255);
    const AGENT_COLOR: egui::Color32 = egui::Color32::from_rgb(128, 140, 255);
    const SYSTEM_COLOR: egui::Color32 = egui::Color32::from_rgb(228, 240, 115);

    pub fn init(
        sender: mpsc::Sender<BackendCommand>,
        receiver: mpsc::Receiver<FrontendRequest>,
    ) -> Self {
        Self {
            current_exchange: CurrentExchange::default(),
            sender: sender.into(),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn current_exchange_panel(&mut self, ui: &mut egui::Ui) {
        use egui::text::LayoutJob;
        let mut job = LayoutJob::default();
        while let Ok(response) = self.receiver.to_owned().lock().unwrap().try_recv() {
            match response {
                FrontendRequest::Message(message) => job.append(
                    &format!("{}", message),
                    0.0,
                    egui::TextFormat {
                        color: Self::AGENT_COLOR,
                        ..Default::default()
                    },
                ),
                FrontendRequest::Done => job.append(
                    &format!("\n"),
                    0.0,
                    egui::TextFormat {
                        ..Default::default()
                    },
                ),
            }
        }
        ui.label(job);
    }

    #[tracing::instrument(name = "Get stream response from frontend", skip(self, ctx, backend))]
    fn spawn_completion_stream_on_backend(&mut self, ctx: &egui::Context, backend: &AppBackend) {
        let user_input = self.current_exchange.user_input_field.to_owned();
        ctx.request_repaint();
        self.sender
            .try_send(BackendCommand::from(user_input))
            .expect("Failed to send to backend");
        backend.spawn_user_prompt();
        self.current_exchange.user_input_field.clear();
    }

    pub fn user_input_window(&mut self, backend: &AppBackend, ctx: &egui::Context) {
        use super::INITAL_WINDOW_SIZE;
        let x = INITAL_WINDOW_SIZE.0 - (INITAL_WINDOW_SIZE.0 / 4.0);
        let y = INITAL_WINDOW_SIZE.1 - (INITAL_WINDOW_SIZE.1 * 0.15);
        egui::Window::new("")
            .title_bar(false)
            .default_pos((x, y))
            .resizable(false)
            .show(ctx, |ui| {
                let text_edit =
                    egui::TextEdit::multiline(&mut self.current_exchange.user_input_field)
                        .desired_rows(4)
                        .frame(false)
                        .lock_focus(true)
                        .horizontal_align(eframe::emath::Align::Min);
                // let enter_button = egui::Button::new(">>");
                let text_edit_handle = ui.add(text_edit);

                if text_edit_handle.has_focus()
                    && ui.input(|i| i.modifiers.shift_only() && i.key_pressed(egui::Key::Enter))
                {
                    self.current_exchange.user_input_field =
                        format!("{}\n", self.current_exchange.user_input_field);
                } else if text_edit_handle.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && !self.current_exchange.user_input_field.trim().is_empty()
                {
                    self.spawn_completion_stream_on_backend(ctx, backend);
                }
                // if ui.add(enter_button).clicked() {
                // }
            });
    }

    pub fn buffer_panel(&self, backend: &AppBackend, ui: &mut egui::Ui) {
        use egui::text::LayoutJob;
        let mut buffer_job = LayoutJob::default();

        let active_buffer = backend
            .buffer()
            .expect("Failed to get agent buffer from the backend");
        let message_vector_ref: &MessageVector = &*active_buffer;

        for message in message_vector_ref.as_ref().iter() {
            match message.role().as_str() {
                "user" => buffer_job.append(
                    &format!(
                        "User: {}\n",
                        message.content().expect("Failed to get ai message content")
                    ),
                    0.0,
                    egui::TextFormat {
                        color: Self::USER_COLOR,
                        ..Default::default()
                    },
                ),
                "assistant" => buffer_job.append(
                    &format!(
                        "Agent: {}\n",
                        message.content().expect("Failed to get ai message content")
                    ),
                    0.0,
                    egui::TextFormat {
                        color: Self::AGENT_COLOR,
                        ..Default::default()
                    },
                ),
                "system" => buffer_job.append(
                    &format!(
                        "System: {}\n",
                        message
                            .content()
                            .expect("Failed to get system message content")
                    ),
                    0.0,
                    egui::TextFormat {
                        color: Self::SYSTEM_COLOR,
                        ..Default::default()
                    },
                ),
                _ => {}
            }
        }

        ui.label(buffer_job);
    }
}
