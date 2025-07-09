use eframe::{CreationContext, Frame, NativeOptions};
use egui::{Context, Response, Slider};
use egui_mvvm::cheap_state::{CheapState, CheapStateChangeDetector, CheapStateHandle};
use egui_mvvm::state::{State, StateChangeDetector, StateHandle};
use egui_mvvm::task_pool::TaskPool;
use egui_mvvm::view_model::{
    request_repaint_on_change, EguiViewModelExt, EguiViewModelsExt, ViewModel, ViewModelErased,
    ViewModelMutRef,
};
use egui_mvvm::ChangeDetector;
use std::pin::Pin;
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
            ctx.memory_ui(ui);

            ui.label(format!("{:?}", std::time::Instant::now()));

            let view_model = ui.fetch_model::<DemoViewModel>();
            DemoView {
                view_model: view_model.get_mut(),
            }
            .show(ui)
        });
    }
}

struct DemoView<'a> {
    view_model: ViewModelMutRef<'a, DemoViewModel>,
}

impl DemoView<'_> {
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
                        1.0..=10.0,
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

#[derive(Default)]
pub struct DemoViewModel {
    pub task_poll: TaskPool,
    pub status: CheapState<Option<Status>>,
    pub error: CheapState<Option<Error>>,
    pub text: State<String>,
    pub jitter: CheapState<f32>,
    pub duration: CheapState<f32>,
}
impl DemoViewModel {
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
            this.error.send_value(None);

            let duration = *this.duration.value();
            let timestep = 1.0 / 30.0;
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
        })
    }
}

pub struct DemoViewModelModel {
    pub status: CheapStateHandle<Option<Status>>,
    pub error: CheapStateHandle<Option<Error>>,
    pub text: StateHandle<String>,
    pub jitter: CheapStateHandle<f32>,
    pub duration: CheapStateHandle<f32>,
}

impl ViewModelErased for DemoViewModel {
    fn task_pool(&self) -> &TaskPool {
        &self.task_poll
    }

    fn latch_state(&mut self) {
        self.status.latch_value();
        self.error.latch_value();
        self.text.latch_value();
        self.jitter.latch_value();
        self.duration.latch_value();
    }

    fn change_detector_boxed(&self) -> Box<dyn ChangeDetector> {
        Box::new(self.change_detector())
    }
}
impl ViewModel for DemoViewModel {
    type Model = DemoViewModelModel;
    type ChangeDetector = DemoViewModelChangeDetector;

    fn make_model(&self) -> Self::Model {
        DemoViewModelModel {
            status: self.status.handle(),
            error: self.error.handle(),
            text: self.text.handle(),
            jitter: self.jitter.handle(),
            duration: self.duration.handle(),
        }
    }

    fn change_detector(&self) -> Self::ChangeDetector {
        DemoViewModelChangeDetector {
            status: self.status.change_detector(),
            error: self.error.change_detector(),
            text: self.text.change_detector(),
            jitter: self.jitter.change_detector(),
            duration: self.duration.change_detector(),
        }
    }
}

#[derive(Clone)]
pub struct DemoViewModelChangeDetector {
    status: CheapStateChangeDetector<Option<Status>>,
    error: CheapStateChangeDetector<Option<Error>>,
    text: StateChangeDetector<String>,
    jitter: CheapStateChangeDetector<f32>,
    duration: CheapStateChangeDetector<f32>,
}

impl ChangeDetector for DemoViewModelChangeDetector {
    fn wait_for_change(&self) -> Pin<Box<dyn Future<Output = Option<()>> + Send + 'static>> {
        let this = self.clone();
        Box::pin(async move {
            tokio::select! {
                res = this.status.wait_for_change() => res,
                res = this.error.wait_for_change() => res,
                res = this.text.wait_for_change() => res,
                res = this.jitter.wait_for_change() => res,
                res = this.duration.wait_for_change() => res
            }
        })
    }
}
