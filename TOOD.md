```rust
use dummy::DummyViewModel;

enum PostStatus {
    Uploading(f32),
    Success,
    Failure
}

view_model! {
    #[view]
    pub struct CommentView {
        #[viewmodel]
        comment: CommentViewModel,
        #[viewmodel]
        dummy: DummyViewModel,
   }

    #[model(default)]
    #[derive(Debug)]
    struct CommentViewModel {
        post_status: StateStream<Option<PostStatus>> = None,
        post_content: MutableState<String> = "".to_string(),
    };
}

impl CommentViewModel {
    pub fn send_content(&self) {
        if self.post_content.get().is_empty() {
            return;
        }

        self.spawn(|model| async {
            // do async things here
            tokio::time::sleep(Duration::from_secs(5)).await;
            model.post_status.set_value(Some(PostStatus::Success));
        })
    }
}

impl CommentView {
    pub fn show(&mut self, ui: &mut egui::Ui) -> Response {
        ui.text_edit_singleline(&mut self.view_model.post_content)

        if ui.button("Submit") {
            self.view_model.send_content();
        }

        if let Some(post_status) = self.view_model.post_status {
            ui.vertical(|ui| {
                ui.label("Status")
                ui.label(format!("{:?}", post_status));
            });
        }
    }
}
```

