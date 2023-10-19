use eframe::{
    egui::{self, RichText},
    epaint::{Color32, FontId},
};
use espionox::context::memory::CachingMechanism;

#[derive(Debug)]
pub struct CachingMechanismUi {
    options_open: bool,
    mech: CachingMechanism,
    limit: f32,
    long_term_enabled: bool,
    mech_replacement: Option<CachingMechanism>,
}

impl From<CachingMechanism> for CachingMechanismUi {
    fn from(value: CachingMechanism) -> Self {
        let mech = value;
        let limit = mech.limit() as f32;
        let long_term_enabled = match mech {
            CachingMechanism::SummarizeAtLimit { save_to_lt, .. } => save_to_lt,
            _ => false,
        };
        Self {
            options_open: false,
            mech,
            limit,
            long_term_enabled,
            mech_replacement: None,
        }
    }
}

impl CachingMechanismUi {
    pub fn mech_name(&self) -> String {
        String::from(match &self.mech {
            CachingMechanism::Forgetful => "Forgetful",
            CachingMechanism::SummarizeAtLimit { .. } => "SummarizeAtLimit",
        })
    }

    pub fn caching_mechanism(&self) -> &CachingMechanism {
        &self.mech
    }

    fn change_to_replacement(&mut self) {
        if let Some(new_mech) = self.mech_replacement.take() {
            let mech = new_mech;
            let limit = mech.limit() as f32;
            let long_term_enabled = mech.long_term_enabled();

            self.mech = mech;
            self.limit = limit;
            self.long_term_enabled = long_term_enabled;
        }
    }

    fn options_display(&mut self, ui: &mut egui::Ui) {
        ui.indent("CachingOptions", |ui| {
            if let None = self.mech_replacement {
                self.mech_replacement = Some(CachingMechanism::Forgetful);
            }
            ui.radio_value(
                &mut self.mech_replacement,
                Some(CachingMechanism::Forgetful),
                "Forgetful",
            );
            ui.radio_value(
                &mut self.mech_replacement,
                Some(CachingMechanism::default_summary_at_limit()),
                "SummarizeAtLimit",
            );
            if self.mech_replacement != Some(CachingMechanism::Forgetful) {
                let upper_bounds = 100.0;
                let lower_bounds = 10.0;
                ui.add(
                    egui::Slider::new(&mut self.limit, lower_bounds..=upper_bounds)
                        .text("Cache size limit"),
                );
                ui.checkbox(&mut self.long_term_enabled, "Save to LTM");
            }

            if ui.button("💾").clicked() {
                let mech = &self.mech_replacement;
                self.mech_replacement = match mech {
                    Some(CachingMechanism::Forgetful) => Some(CachingMechanism::Forgetful),
                    Some(CachingMechanism::SummarizeAtLimit { .. }) => {
                        Some(CachingMechanism::SummarizeAtLimit {
                            limit: self.limit as usize,
                            save_to_lt: self.long_term_enabled,
                        })
                    }
                    None => None,
                };
                self.change_to_replacement();
                self.options_open = false;
            }
        });
    }

    pub fn overview_display(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.colored_label(
                Color32::GOLD,
                format!("Caching limit: {}", &self.mech.limit()),
            );

            if self.long_term_enabled {
                ui.colored_label(
                    Color32::GOLD,
                    RichText::new("LTM").font(FontId::proportional(9.0)),
                );
            } else {
            }
        });

        if ui
            .selectable_label(self.options_open, "Change Mechanism")
            .clicked()
        {
            self.options_open = !self.options_open;
        }

        if self.options_open {
            self.options_display(ui);
        }
    }
}
