use eframe::egui;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    Chat,
    Settings,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<State> for egui::WidgetText {
    fn from(value: State) -> Self {
        Self::RichText(egui::RichText::new(value.to_string()))
    }
}

impl Default for State {
    fn default() -> Self {
        Self::Chat
    }
}

impl State {
    pub fn all() -> Vec<Self> {
        vec![State::Chat, State::Settings]
    }
}
