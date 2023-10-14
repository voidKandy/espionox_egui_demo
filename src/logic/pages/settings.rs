use super::{super::backend::AppBackend, egui, PageDisplay};

use espionox::context::memory::MessageVector;

#[derive(Debug)]
pub struct SettingsPage {
    // agent_settings: AgentSettingsWrapper,
}

#[derive(Debug)]
struct AgentSettingsWrapper {
    // settings: AgentSettings,
    initial_prompt_change: MessageVector,
}

// impl From<AgentSettings> for AgentSettingsWrapper {
//     fn from(settings: AgentSettings) -> Self {
//         Self {
//             settings,
//             initial_prompt_change: MessageVector::init(),
//         }
//     }
// }

trait SettingsInterface {
    fn ui(&mut self, _ui: &mut egui::Ui) {}
}

impl From<&AppBackend> for SettingsPage {
    fn from(_backend: &AppBackend) -> Self {
        // THIS IS NOT A FINISHED IMPLEMENTATION!
        // HOW AGENTS ARE GRABBED NEEDS TO BE BUILT FIRST
        Self {
            // agent_settings: AgentSettings::default().into(),
        }
    }
}

impl SettingsPage {
    // fn all_settings(&mut self) -> Vec<&mut dyn SettingsInterface> {
    //     vec![&mut self.agent_settings]
    // }
}

impl SettingsInterface for AgentSettingsWrapper {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut sys_prompt_visible = false;
        // ui.vertical(|ui| {
        //     ui.label("Agent Settings");
        //     let show_prompt_button =
        //         ui.selectable_label(sys_prompt_visible, "System Prompt:".to_string());
        //
        //     let mut init_prompt = self.settings.init_prompt.clone();
        //     // let mut system_prompt = init_prompt
        //     //     .as_ref()
        //     //     .iter()
        //     //     .map(|m| m.content().unwrap())
        //     //     .collect::<Vec<String>>()
        //     //     .join("\n");
        //     //
        //     let messages: Vec<String> = init_prompt
        //         .as_ref()
        //         .into_iter()
        //         .map(|message| message.content().unwrap())
        //         .collect();
        //
        //     // let mut text_edits: Vec<egui::TextEdit> = vec![];
        //
        //     // for message in messages.into_iter() {
        //     let mut mes = message.clone();
        //     text_edits.push(
        //         egui::TextEdit::multiline(&mut mes)
        //             .horizontal_align(eframe::emath::Align::Min)
        //             .desired_width(f32::INFINITY)
        //             .interactive(true),
        //     );
        //     // }
        //
        //     // let prompt_width = ui.available_size().x * 0.6;
        //     // let prompt_height = ui.available_size().y * 0.85;
        //     // let prompt_size: (f32, f32) = (prompt_width, prompt_height);
        //
        //     if show_prompt_button.enabled() {
        //         text_edits.into_iter().for_each(|textedit| {
        //             ui.add(textedit);
        //         })
        //     }
        //     if show_prompt_button.clicked() {
        //         // self.show_agent_settings(ui);
        //         // system_prompt_edit.frame(true).interactive(false);
        //         sys_prompt_visible = !sys_prompt_visible;
        //     }
        // });
    }
}

impl SettingsPage {
    pub fn display(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            // for settings in self.all_settings() {
            // settings.ui(ui);
            // let (rect, _response) =
            //     ui.allocate_at_least(egui::vec2(64.0, 32.0), egui::Sense::hover());
            // ui.painter().rect(
            //     rect,
            //     8.0,
            //     egui::Color32::from_gray(64),
            //     egui::Stroke::new(0.0, egui::Color32::WHITE),
            // );
            // }
        });
    }
}
