use egui::{Color32, Response, Slider, Ui};
use egui_mvvm::val_state::ValState;
use egui_mvvm::view_model;

view_model! {
    #[view]
    pub struct ThemeEditor {
        #[viewmodel]
        pub theme: &mut AppTheme,
    }

    #[viewmodel(default)]
    pub struct AppTheme {
        pub brand_primary: ValState<Color32> = Color32::from_rgb(0x61, 0x19, 0xBF),
        pub brand_secondary: ValState<Color32> = Color32::from_rgb(0x7B, 0xC2, 0x1F),
        pub brand_neutral: ValState<Color32> = Color32::from_rgb(0xFA, 0xFC, 0xFC),
        pub primary_text_on_neutral: ValState<Color32> = Color32::from_rgb(0x22, 0x26, 0x25),
        pub secondary_text_on_neutral: ValState<Color32> = Color32::from_rgb(0x6F, 0x78, 0x77),
        pub spacing_unit: ValState<f32> = 8.0,
    }
}

impl ThemeEditor<'_> {
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Brand Primary: ");
                self.theme
                    .brand_primary
                    .with_mut_for_ui(ui, |ui, val| ui.color_edit_button_srgba(val));
            });

            ui.horizontal(|ui| {
                ui.label("Brand Secondary: ");
                self.theme
                    .brand_secondary
                    .with_mut_for_ui(ui, |ui, val| ui.color_edit_button_srgba(val));
            });

            ui.horizontal(|ui| {
                ui.label("Brand Neutral: ");
                self.theme
                    .brand_neutral
                    .with_mut_for_ui(ui, |ui, val| ui.color_edit_button_srgba(val));
            });

            ui.horizontal(|ui| {
                ui.label("Primary Text on Neutral: ");
                self.theme
                    .primary_text_on_neutral
                    .with_mut_for_ui(ui, |ui, val| ui.color_edit_button_srgba(val));
            });

            ui.horizontal(|ui| {
                ui.label("Secondary Text on Neutral: ");
                self.theme
                    .secondary_text_on_neutral
                    .with_mut_for_ui(ui, |ui, val| ui.color_edit_button_srgba(val));
            });

            ui.horizontal(|ui| {
                ui.label("Spacing Unit: ");
                self.theme
                    .spacing_unit
                    .with_mut_for_ui(ui, |ui, val| ui.add(Slider::new(val, 1.0..=100.0)));
            });
        })
        .response
    }
}
