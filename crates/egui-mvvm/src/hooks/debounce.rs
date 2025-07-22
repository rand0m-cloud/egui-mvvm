use crate::hooks::effect::UseEffect;
use crate::hooks::state::UseState;
use egui::Ui;
use std::time::Duration;

pub fn use_debounce<T>(val: T, delay: Duration, ui: &mut Ui) -> T
where
    T: PartialEq + Clone + Send + Sync + 'static,
{
    let state = ui.use_val_state_or_insert(|| val.clone());
    {
        let handle = (*state.get()).handle();
        ui.use_effect((val, delay), |(val, delay)| {
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                handle.send_update(|v| *v = val);
            })
        });
    }

    state.get().value().clone()
}

pub trait UseDebounce {
    fn use_debounce<T>(&mut self, val: T, delay: Duration) -> T
    where
        T: PartialEq + Clone + Send + Sync + 'static;
}

impl UseDebounce for egui::Ui {
    fn use_debounce<T>(&mut self, val: T, delay: Duration) -> T
    where
        T: PartialEq + Clone + Send + Sync + 'static,
    {
        use_debounce::<T>(val, delay, self)
    }
}
