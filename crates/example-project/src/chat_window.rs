use crate::chat_service::{ChannelId, ChatMessageId, ChatService};
use crate::theme::AppTheme;
use egui::{Response, RichText, Sense, Stroke, Ui};
use egui_mvvm::ref_state::RefState;
use egui_mvvm::view_model;
use jiff::tz::TimeZone;

view_model! {
    #[view]
    pub struct ChatWindow {
        pub channel_id: ChannelId,
        #[viewmodel]
        pub chat_service: &ChatService,
        #[viewmodel]
        pub chat_window: &mut ChatWindowViewModel,
        #[viewmodel]
        pub theme: &AppTheme,
    }

    #[view]
    struct ChatMessageView {
        message_id: ChatMessageId,
        #[viewmodel]
        chat_service: &ChatService,
        #[viewmodel]
        theme: &AppTheme,
    }

    #[viewmodel(default)]
    pub struct ChatWindowViewModel {
        message_box: RefState<String> = "".to_string(),
    }
}

impl ChatWindow<'_> {
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let channel = self
            .chat_service
            .channel_name(self.channel_id)
            .unwrap_or_default();

        ui.vertical(|ui| {
            ui.add_space(*self.theme.spacing_unit.value());
            ui.horizontal(|ui| {
                ui.add_space(*self.theme.spacing_unit.value());
                ui.label(
                    RichText::new(&*channel)
                        .color(*self.theme.primary_text_on_neutral.value())
                        .strong(),
                );
            });

            ui.add_space(*self.theme.spacing_unit.value());
            ui.scope(|ui| {
                ui.visuals_mut().widgets.noninteractive.bg_stroke = Stroke::new(
                    0.25 * self.theme.spacing_unit.value(),
                    *self.theme.brand_primary.value(),
                );
                ui.separator();
            });
            ui.add_space(2.0 * self.theme.spacing_unit.value());

            let mut first_message = true;
            ui.vertical(|ui| {
                self.chat_service
                    .channel_message_ids(self.channel_id, |messages| {
                        for message_id in messages.iter().cloned() {
                            if !first_message {
                                ui.add_space(2.0 * self.theme.spacing_unit.value());
                            } else {
                                first_message = false;
                            }

                            ui.horizontal(|ui| {
                                ui.add_space(*self.theme.spacing_unit.value());
                                ChatMessageView {
                                    message_id,
                                    chat_service: self.chat_service.handle().clone().get(),
                                    theme: self.theme.handle().clone().get(),
                                }
                                .show(ui);
                            });
                        }
                    })
            });
        })
        .response
    }
}

impl ChatMessageView<'_> {
    pub fn show(&mut self, ui: &mut Ui) {
        self.chat_service.message(self.message_id, |msg| {
            let timestamp = msg
                .timestamp
                .to_zoned(TimeZone::system())
                .strftime("%I:%M %p");

            ui.horizontal(|ui| {
                // Icon
                let (resp, painter) = ui.allocate_painter(
                    egui::Vec2::from([4.0 * *self.theme.spacing_unit.value(); 2]),
                    Sense::empty(),
                );
                painter.circle(
                    resp.rect.center(),
                    1.75 * self.theme.spacing_unit.value(),
                    *self.theme.brand_primary.value(),
                    Stroke::new(
                        0.25 * self.theme.spacing_unit.value(),
                        *self.theme.brand_secondary.value(),
                    ),
                );
                ui.add_space(*self.theme.spacing_unit.value());

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&msg.author)
                                .color(*self.theme.primary_text_on_neutral.value())
                                .strong(),
                        );
                        ui.add_space(*self.theme.spacing_unit.value());
                        ui.colored_label(
                            *self.theme.secondary_text_on_neutral.value(),
                            timestamp.to_string(),
                        );
                    });
                    ui.add_space(0.25 * self.theme.spacing_unit.value());
                    ui.colored_label(*self.theme.primary_text_on_neutral.value(), &msg.message);
                });
            });
        })
    }
}
