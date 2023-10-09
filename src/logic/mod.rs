pub mod backend;
pub mod comms;
pub mod pages;
pub mod state;

use self::{
    backend::AppBackend,
    comms::FrontendComms,
    pages::{ChatPage, SettingsPage},
    state::State,
};
use eframe::egui;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct MainApplication {
    state: State,
    chat_page: ChatPage,
    settings_page: SettingsPage,
    frontend: FrontendComms,
    backend: AppBackend,
}

impl Default for MainApplication {
    fn default() -> Self {
        let rt = tokio::runtime::Runtime::new().expect("Unable to create Runtime");

        let _enter = rt.enter();

        std::thread::spawn(move || {
            rt.block_on(async {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            })
        });
        let (frontend_to_backend_sender, frontend_to_backend_receiver) = mpsc::channel(100);
        let (backend_to_frontend_sender, backend_to_frontend_receiver) = mpsc::channel(100);

        let frontend =
            FrontendComms::init(frontend_to_backend_sender, backend_to_frontend_receiver);
        let backend = AppBackend::init(backend_to_frontend_sender, frontend_to_backend_receiver);

        Self {
            state: State::default(),
            chat_page: ChatPage::init(),
            settings_page: SettingsPage::from(&backend),
            frontend,
            backend,
        }
    }
}

pub const INITAL_WINDOW_SIZE: (f32, f32) = (1280.0, 640.0);

impl eframe::App for MainApplication {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.top_bar_ui(ctx, frame);
        Self::display_main_window(ctx, frame, |ui| match self.state {
            State::Chat => {
                // if !self.backend.max_chat_threads_spawned() {
                // self.backend.spawn_chat_threads().unwrap();
                // }
                self.chat_page.display_current_chat(&self.frontend, ui);
                // let _ = self.backend.listen_for_commands();
            }
            State::Settings => self.settings_page.display(ui),
        });
    }
}

impl MainApplication {
    pub fn run() -> Result<(), eframe::Error> {
        let (x, y) = INITAL_WINDOW_SIZE;
        let options = eframe::NativeOptions {
            decorated: false,
            transparent: true,
            min_window_size: Some(egui::vec2(x, y)),
            initial_window_size: Some(egui::vec2(x, y)),
            ..Default::default()
        };
        eframe::run_native(
            "espionox",
            options,
            Box::new(|_cc| Box::<MainApplication>::default()),
        )
    }

    pub fn display_main_window(
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        add_contents: impl FnOnce(&mut egui::Ui),
    ) {
        use egui::*;

        CentralPanel::default().show(ctx, |ui| {
            catppuccin_egui::set_theme(&ctx, catppuccin_egui::MACCHIATO);
            let app_rect = ui.max_rect();
            let title_bar_height = 10.0;
            let title_bar_rect = {
                let mut rect = ui.max_rect();
                rect.max.y = rect.min.y + title_bar_height;
                rect
            };
            let content_rect = {
                let mut rect = app_rect;
                rect.min.y = title_bar_rect.max.y;
                rect
            }
            .shrink(4.0);

            // self.top_bar_ui(ui, frame, title_bar_rect, title);
            let mut content_ui = ui.child_ui(content_rect, *ui.layout());
            let content_response = ui.interact(content_rect, Id::new("main_rect"), Sense::click());
            if content_response.is_pointer_button_down_on() {
                frame.drag_window();
            }

            add_contents(&mut content_ui);
        });
    }

    fn top_bar_ui(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        use egui::*;
        TopBottomPanel::top("top_bar")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let mut current_state = self.state;
                        for (i, state) in State::all().into_iter().enumerate() {
                            if ui
                                .selectable_label(current_state == state, state.to_string())
                                .clicked()
                            {
                                current_state = state;
                            }
                            if i != State::all().len() - 1 {
                                ui.separator();
                            }
                        }
                        self.state = current_state;
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 0.2;
                        ui.visuals_mut().button_frame = false;
                        ui.add_space(8.0);
                        Self::close_maximize_minimize(ui, frame);
                    });
                });
            });
    }

    fn close_maximize_minimize(ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        use egui::{Button, RichText};

        let button_height = 12.0;

        let close_response = ui
            .add(Button::new(RichText::new("❌").size(button_height)))
            .on_hover_text("Close the window");
        if close_response.clicked() {
            frame.close();
        }

        if frame.info().window_info.maximized {
            let maximized_response = ui
                .add(Button::new(RichText::new("🗗").size(button_height)))
                .on_hover_text("Restore window");
            if maximized_response.clicked() {
                frame.set_maximized(false);
            }
        } else {
            let maximized_response = ui
                .add(Button::new(RichText::new("🗗").size(button_height)))
                .on_hover_text("Maximize window");
            if maximized_response.clicked() {
                frame.set_maximized(true);
            }
        }

        let minimized_response = ui
            .add(Button::new(RichText::new("🗕").size(button_height)))
            .on_hover_text("Minimize the window");
        if minimized_response.clicked() {
            frame.set_minimized(true);
        }
    }
}
