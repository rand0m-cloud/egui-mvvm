use eframe::{CreationContext, Frame, NativeOptions};
use egui::{Context, Response};
use egui_mvvm::state_stream::{
    ChangeDetector, StateStream, StateStreamChangeDetector, StateStreamHandle,
};
use egui_mvvm::task_pool::TaskPool;
use egui_mvvm::view_model::ViewModel;
use std::time::Duration;
use rand::{random, random_range};

#[tokio::main]
async fn main() {
        let view_model = CommentViewModel::new();
        let change_detector = view_model.change_detector();

        eframe::run_native(
            "egui-mvvm",
            NativeOptions::default(),
            Box::new(move |creation: &CreationContext| {
                Ok(EguiApp::new(
                    &creation.egui_ctx,
                    change_detector,
                    view_model,
                ))
            }),
        )
        .unwrap()
}

struct EguiApp {
    view_model: CommentViewModel,
}

impl EguiApp {
    pub fn new(
        ctx: &egui::Context,
        mut change_detector: impl ChangeDetector,
        view_model: CommentViewModel,
    ) -> Box<Self> {
        // Send Repaint Requests when the ViewModel changes.
        {
            let ctx = ctx.clone();
            tokio::spawn(async move {
                loop {
                    change_detector.wait_for_change().await;
                    ctx.request_repaint();
                }
            });
        }

        Box::new(Self { view_model })
    }
}

impl eframe::App for EguiApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.view_model.latch_state();

            ui.label(format!("{:?}", std::time::Instant::now()));
            CommentView {
                view_model: &mut self.view_model,
            }
            .show(ui)
        });
    }
}

struct CommentView<'a> {
    view_model: &'a mut CommentViewModel,
}

impl CommentView<'_> {
    pub fn show(&mut self, ui: &mut egui::Ui) -> Response {
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.view_model.post_content);
            if ui.button("Submit").clicked() {
                self.view_model.send_comment();
            }

            if let Some(post_status) = &*self.view_model.post_status.value() {
                ui.vertical(|ui| {
                    ui.heading("Result:");
                    ui.label(format!("{:?}", post_status));
                });
            }
        })
        .response
    }
}

#[derive(Debug)]
enum PostStatus {
    Uploading(f32),
    Success,
}

struct CommentViewModel {
    task_poll: TaskPool,
    post_status: StateStream<Option<PostStatus>>,
    pub post_content: String,
}

//#[derive(ViewModel)]
//struct CommentViewModel {
//    post_status: StateStream<Option<PostStatus>> = None,
//    post_content: String
//}

pub struct CommentViewModelModel {
    post_status: StateStreamHandle<Option<PostStatus>>,
}

pub struct CommentViewModelChangeDetector {
    post_status: StateStreamChangeDetector<Option<PostStatus>>,
}

impl ChangeDetector for CommentViewModelChangeDetector {
    async fn wait_for_change(&mut self) -> Option<()> {
        self.post_status.wait_for_change().await
    }
}

impl ViewModel for CommentViewModel {
    type Model = CommentViewModelModel;
    fn task_pool(&self) -> &TaskPool {
        &self.task_poll
    }

    fn latch_state(&mut self) {
        self.post_status.latch_value();
    }
    fn has_changed(&self) -> bool {
        self.post_status.has_changed()
    }

    fn make_model(&self) -> Self::Model {
        CommentViewModelModel {
            post_status: self.post_status.handle(),
        }
    }

    fn change_detector(&self) -> Self::ChangeDetector {
        CommentViewModelChangeDetector {
            post_status: self.post_status.change_detector(),
        }
    }

    type ChangeDetector = CommentViewModelChangeDetector;
}

impl CommentViewModel {
    pub fn new() -> Self {
        Self {
            task_poll: TaskPool::new(),
            post_status: StateStream::new(None),
            post_content: String::new(),
        }
    }

    pub fn send_comment(&self) {
        let mut progress = 0.0;
        self.post_status
            .set_value(Some(PostStatus::Uploading(progress)));
        self.spawn(|this| async move {
            while progress < 1.0 {
                progress += 0.01;
                println!("progress: {}", progress);

                tokio::time::sleep(Duration::from_millis(1)).await;
                this.post_status
                    .set_value(Some(PostStatus::Uploading(progress)));
            }

            println!("success");
            this.post_status.set_value(Some(PostStatus::Success));
        })
    }
}
