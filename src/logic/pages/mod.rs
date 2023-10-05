pub mod chat;
pub mod settings;

pub use chat::ChatPage;
pub use settings::SettingsPage;

use eframe::egui;

use super::MainApplication;

//up next...
pub trait PageDisplay {
    fn display(&mut self, app: MainApplication, _ui: &mut egui::Ui) {}
}
