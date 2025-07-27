use crate::chat_service::ChatService;
use crate::theme::AppTheme;
use egui::{Margin, Response, Stroke, Ui};
use egui_mvvm::view_model;

view_model! {
    #[view]
    pub struct ChatChannels {
        #[viewmodel]
        pub chat_service: &ChatService,
        #[viewmodel]
        pub theme: &AppTheme,
    }
}

impl ChatChannels<'_> {
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        egui::Frame {
            fill: *self.theme.brand_primary.value(),
            inner_margin: Margin::same(8),
            ..egui::Frame::NONE
        }
        .show(ui, |ui| {
            ui.allocate_ui(
                egui::Vec2::new(
                    16.0 * self.theme.spacing_unit.value(),
                    ui.available_height(),
                ),
                |ui| {
                    ui.vertical(|ui| {
                        ui.colored_label(*self.theme.primary_text_on_neutral.value(), "Channels");
                        ui.scope(|ui| {
                            ui.visuals_mut().widgets.noninteractive.bg_stroke = Stroke::new(
                                0.25 * self.theme.spacing_unit.value(),
                                *self.theme.brand_primary.value(),
                            );
                            ui.separator();
                        });

                        self.chat_service.channels(|channel_id| {
                            let name = self
                                .chat_service
                                .channel_name(channel_id)
                                .unwrap_or_default();
                            dbg!(&name);
                            ui.colored_label(
                                *self.theme.primary_text_on_neutral.value(),
                                format!("# {}", name),
                            );
                        });

                        ui.add_space(ui.available_height());
                    });
                },
            )
        })
        .response
    }
}
