use eframe::{CreationContext, Frame, NativeOptions};
use egui::{Context, Response, Slider};
use egui_mvvm::hooks::debounce::use_debounce;
use egui_mvvm::ref_state::RefState;
use egui_mvvm::val_state::ValState;
use egui_mvvm::view_model;
use egui_mvvm::view_model::{
    EguiViewModelExt, EguiViewModelsExt, ViewModel, request_repaint_on_change,
};
use std::sync::Arc;
use std::time::Duration;

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

            let view_model = ui.fetch_model::<SearchViewModel>();
            SearchView {
                view_model: view_model.get_mut(),
            }
            .show(ui)
        });
    }
}

view_model! {
    #[view]
    pub struct SearchView {
        #[viewmodel]
        pub view_model: &mut SearchViewModel,
    }

    #[viewmodel(default)]
    pub struct SearchViewModel {
        pub search: RefState<String> = "default value".to_string(),
        pub debounce_millis: ValState<u64> = 300,
    }
}

impl SearchView<'_> {
    pub fn show(&mut self, ui: &mut egui::Ui) -> Response {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .text_edit_singleline(&mut *self.view_model.search.value_mut_untracked())
                    .changed()
                {
                    self.view_model.search.mark_changed();
                }

                ui.vertical(|ui| {
                    ui.label("Debounce in ms");

                    if ui
                        .add(Slider::new(
                            self.view_model.debounce_millis.value_mut_untracked(),
                            10..=1000,
                        ))
                        .changed()
                    {
                        self.view_model.debounce_millis.mark_changed();
                    }
                });
            });

            let debounced = use_debounce(
                Arc::<str>::from(self.view_model.search.value().as_str()),
                Duration::from_millis(*self.view_model.debounce_millis.value()),
                ui,
            );
            ui.label(format!("Debounced: {:?}", debounced));
        })
        .response
    }
}
