use crate::ref_state::RefState;
use crate::val_state::ValState;
use crate::view_model::{EguiViewModelExt, ViewModelHandle};
use egui::Ui;

pub trait UseState {
    fn use_ref_state<T>(&mut self) -> ViewModelHandle<RefState<T>>
    where
        T: Default + Send + Sync + 'static;
    fn use_ref_state_or_insert<T>(&mut self, f: impl FnOnce() -> T) -> ViewModelHandle<RefState<T>>
    where
        T: Send + Sync + 'static;
    fn use_val_state<T>(&mut self) -> ViewModelHandle<ValState<T>>
    where
        T: Default + Clone + Send + Sync + 'static;

    fn use_val_state_or_insert<T>(&mut self, f: impl FnOnce() -> T) -> ViewModelHandle<ValState<T>>
    where
        T: Clone + Send + Sync + 'static;
}

impl UseState for Ui {
    fn use_ref_state<T>(&mut self) -> ViewModelHandle<RefState<T>>
    where
        T: Default + Send + Sync + 'static,
    {
        self.use_ref_state_or_insert(|| T::default())
    }

    fn use_ref_state_or_insert<T>(&mut self, f: impl FnOnce() -> T) -> ViewModelHandle<RefState<T>>
    where
        T: Send + Sync + 'static,
    {
        self.fetch_model_or_insert::<RefState<T>, _>(|| RefState::new(f()))
    }

    fn use_val_state<T>(&mut self) -> ViewModelHandle<ValState<T>>
    where
        T: Default + Clone + Send + Sync + 'static,
    {
        self.use_val_state_or_insert(|| T::default())
    }

    fn use_val_state_or_insert<T>(&mut self, f: impl FnOnce() -> T) -> ViewModelHandle<ValState<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.fetch_model_or_insert::<ValState<T>, _>(|| ValState::new(f()))
    }
}
