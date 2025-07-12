use crate::task_pool::{EguiLocalTaskPool, TaskHandle};
use crate::val_state::ValState;
use crate::view_model::EguiViewModelExt;
use std::pin::Pin;

pub trait UseDisposable<I> {
    fn use_disposable_effect(
        self,
        id: I,
        block: impl FnOnce(I) -> Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>,
    );
}

impl<I> UseDisposable<I> for &mut egui::Ui
where
    I: PartialEq + Clone + Send + Sync + 'static,
{
    fn use_disposable_effect(
        mut self,
        id: I,
        block: impl FnOnce(I) -> Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>,
    ) {
        let state = self.fetch_model_or_insert(|| {
            ValState::<(Option<I>, Option<TaskHandle>)>::new((None, None))
        });
        let state = state.get_mut();

        if state.value().0.as_ref() != Some(&id) {
            if let Some(handle) = &state.value().1 {
                println!("aborting disposable task");
                handle.abort();
            }

            let handle = {
                let value = id.clone();
                let block = block(value);
                println!("spawning task");
                self.local_task_pool().spawn(block)
            };

            println!("dispoable new state");
            state.send_modify(|(state_id, state_task_handle)| {
                *state_id = Some(id);
                *state_task_handle = Some(handle);
            });
        }
    }
}
