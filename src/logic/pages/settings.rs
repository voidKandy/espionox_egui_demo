use eframe::epaint::Color32;

use super::{super::backend::AppBackend, egui, PageDisplay};

#[derive(Debug)]
pub struct GlobalSettings {
    chat_settings: ChatSettings,
}

#[derive(Debug)]
struct ChatSettings {
    user: (String, Color32),
    assistant: (String, Color32),
    system: (String, Color32),
}
