use eframe::{CreationContext, Frame, NativeOptions};
use egui::{Context, Response, Slider};
use egui_mvvm::ref_state::RefState;
use egui_mvvm::val_state::ValState;
use egui_mvvm::view_model;
use egui_mvvm::view_model::{
    request_repaint_on_change, EguiViewModelExt, EguiViewModelsExt, ViewModel,
};
use std::time::{Duration, Instant};

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
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        ctx.memory_mut(|mem| mem.view_models().latch_values());

        egui::CentralPanel::default().show(ctx, |ui| {
            //ctx.memory_ui(ui);

            ui.label(format!("{:?}", std::time::Instant::now()));

            let view_model = ui.fetch_model::<DownloadViewModel>();
            DownloadView {
                view_model: view_model.get_mut(),
            }
            .show(ui)
        });
    }
}

view_model! {
    #[view]
    pub struct DownloadView {
        #[viewmodel]
        pub view_model: &mut DownloadViewModel,
    }

    #[viewmodel(default)]
    pub struct DownloadViewModel {
        pub status: ValState<Option<Status>> = None,
        pub error: ValState<Option<Error>> = None,
        pub text: RefState<String> = "".to_string(),
        pub jitter: ValState<f32> = 0.0,
        pub duration: ValState<f32> = 0.0,
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    Preparing,
    Uploading(f32),
    Success,
}

#[derive(Debug, Clone)]
pub enum Error {
    MissingDuration,
}

impl DownloadView<'_> {
    pub fn show(&mut self, ui: &mut egui::Ui) -> Response {
        ui.vertical(|ui| {
            if let Some(error) = self.view_model.error.value() {
                match error {
                    Error::MissingDuration => ui.label("Duration cannot be zero!"),
                };
            }

            ui.horizontal(|ui| {
                if ui
                    .text_edit_singleline(&mut *self.view_model.text.value_mut_untracked())
                    .changed()
                {
                    self.view_model.text.mark_changed();
                };

                if ui.button("Submit").clicked()
                    || ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    self.view_model.simulate_upload();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Jitter: ");

                if ui
                    .add(Slider::new(
                        &mut *self.view_model.jitter.value_mut_untracked(),
                        1.0..=20.0,
                    ))
                    .changed()
                {
                    self.view_model.jitter.mark_changed()
                }
            });

            ui.horizontal(|ui| {
                ui.label("Duration: ");

                if ui
                    .add(Slider::new(
                        &mut *self.view_model.duration.value_mut_untracked(),
                        0.0..=10.0,
                    ))
                    .changed()
                {
                    self.view_model.jitter.mark_changed()
                }
            });

            if let Some(status) = self.view_model.status.value().as_ref() {
                ui.vertical(|ui| {
                    ui.heading("Result:");
                    match status {
                        Status::Preparing => {
                            ui.label("Preparing upload");
                        }
                        Status::Uploading(progress) => {
                            ui.label(format!("Uploading... ({}%)", (progress * 100.).round()));
                        }
                        Status::Success => {
                            ui.label("Success");
                        }
                    }
                });
            }
        })
        .response
    }
}

impl DownloadViewModel {
    pub fn is_simulating(&self) -> bool {
        matches!(
            self.status.value(),
            Some(Status::Preparing | Status::Uploading(..))
        )
    }

    pub fn simulate_upload(&self) {
        dbg!(self.duration.value(), self.jitter.value());

        if self.is_simulating() {
            return;
        }

        if *self.duration.value() == 0.0 {
            self.error.send_value(Some(Error::MissingDuration));
            return;
        }

        self.status.send_value(Some(Status::Preparing));

        self.spawn(|this| async move {
            let duration = *this.duration.value();
            let timestep = 1.0 / 90.0;
            let mut progress = 0.0;

            tokio::time::sleep(Duration::from_secs(1)).await;

            this.status.send_value(Some(Status::Uploading(0.0)));

            let start = Instant::now();
            while progress < duration {
                let timestamp_millis = (1000.0 * timestep) as u64;
                tokio::time::sleep(Duration::from_millis(rand::random_range(
                    timestamp_millis..=timestamp_millis * this.jitter.latest_value() as u64,
                )))
                .await;

                progress += timestep;
                let normalized = progress / duration;

                this.status.send_value(Some(Status::Uploading(normalized)));
                println!("progress: {}", normalized);
            }

            println!("took {} seconds", start.elapsed().as_secs());
            this.status.send_value(Some(Status::Success));

            tokio::time::sleep(Duration::from_millis(300)).await;
            this.text
                .send_update(|content| *content = content.to_uppercase());
        });
    }
}
