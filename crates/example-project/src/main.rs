use crate::chat_channels::ChatChannels;
use crate::chat_service::{ChannelId, ChatMessage, ChatService};
use crate::chat_window::ChatWindow;
use crate::theme::{AppTheme, ThemeEditor};
use eframe::emath::Align;
use eframe::{CreationContext, NativeOptions};
use egui::{Context, Frame, Layout, Spacing};
use egui_mvvm::hooks::effect::UseEffect;
use egui_mvvm::view_model::{request_repaint_on_change, EguiViewModelExt, EguiViewModelsExt};
use jiff::Timestamp;
use std::time::Duration;

mod chat_channels;
pub mod chat_service;
pub mod chat_window;
pub mod theme;

#[tokio::main]
async fn main() {
    eframe::run_native(
        "egui-mvvm",
        NativeOptions::default(),
        Box::new(move |creation: &CreationContext| Ok(EguiApp::new(&creation.egui_ctx))),
    )
    .unwrap()
}

struct EguiApp {}

impl EguiApp {
    pub fn new(ctx: &Context) -> Box<Self> {
        tokio::spawn(request_repaint_on_change(ctx.clone()));

        Box::new(Self {})
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.memory_mut(|mem| mem.view_models().latch_values());

        root_app(ctx);
    }
}

fn root_app(ctx: &Context) {
    let theme = ctx.fetch_model::<AppTheme>();

    ctx.all_styles_mut(|style| {
        style.spacing = Spacing {
            item_spacing: Default::default(),
            ..Default::default()
        };
    });

    egui::Window::new("Theme Editor").show(ctx, |ui| {
        ThemeEditor {
            theme: theme.get_mut(),
        }
        .show(ui)
    });

    let main_frame = Frame {
        fill: *theme.get().brand_neutral.value(),
        ..Frame::NONE
    };

    egui::CentralPanel::default()
        .frame(main_frame)
        .show(ctx, |ui| {
            let chat_service = ui.fetch_model_or_insert(create_demo_chat_service);
            let channel_id = ChannelId(1);

            ui.use_effect((), |_| {
                let chat_service = chat_service.clone();
                Box::pin(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        chat_service.get().send_message(
                            channel_id,
                            ChatMessage {
                                author: "System".to_string(),
                                message: format!("System message from {}", Timestamp::now()),
                                timestamp: Timestamp::now(),
                            },
                        )
                    }
                })
            });

            ui.allocate_ui_with_layout(
                ui.available_size(),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ChatChannels {
                        chat_service: chat_service.get(),
                        theme: theme.get(),
                    }
                    .show(ui);

                    ChatWindow {
                        channel_id,
                        chat_service: chat_service.get(),
                        chat_window: ui.fetch_model().get_mut(),
                        theme: theme.get(),
                    }
                    .show(ui);
                },
            );
        });
}

fn create_demo_chat_service() -> ChatService {
    let service = ChatService::default();

    let channel = service.create_channel("General");
    service.send_message(
        channel,
        ChatMessage {
            author: "Tony".to_string(),
            message: "waste management".to_string(),
            timestamp: Timestamp::now(),
        },
    );

    service
}
