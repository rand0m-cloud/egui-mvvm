use crate::view_model::{ViewModel, ViewModelLike};
use crate::{ChangeDetector, Stateful};
use egui::{Response, Ui};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use tokio::sync::watch;

/// Use this for state when [`S`] is a trivially copied type and Arc-Mutexing state isn't necessary.
#[derive(Clone)]
pub struct ValState<S> {
    latched: S,
    tx: watch::Sender<S>,
    rx: watch::Receiver<S>,
}

impl<S: Default + Send + Sync + Clone + 'static> Default for ValState<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

pub struct ValStateMutRef<'a, S: Clone> {
    state: &'a mut S,
    tx: watch::Sender<S>,
}

impl<S: Clone> Drop for ValStateMutRef<'_, S> {
    fn drop(&mut self) {
        let _ = self.tx.send(self.state.clone());
    }
}

impl<S: Clone> Deref for ValStateMutRef<'_, S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<S: Clone> DerefMut for ValStateMutRef<'_, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl<S: 'static + Send + Sync + Clone> ValState<S> {
    pub fn new(value: S) -> Self {
        let (tx, rx) = watch::channel(value.clone());
        Self {
            latched: value,
            tx,
            rx,
        }
    }

    pub fn latch_value(&mut self) {
        if self.rx.has_changed().unwrap_or(true) {
            self.latched = self.rx.borrow_and_update().clone();
        }
    }

    pub fn latest_value(&self) -> S {
        self.tx.borrow().clone()
    }

    pub fn value(&self) -> &S {
        &self.latched
    }

    pub fn value_mut(&mut self) -> ValStateMutRef<'_, S> {
        ValStateMutRef {
            state: &mut self.latched,
            tx: self.tx.clone(),
        }
    }

    pub fn value_mut_untracked(&mut self) -> &mut S {
        &mut self.latched
    }

    pub fn send_value(&self, value: S) {
        let _ = self.tx.send(value);
    }

    pub fn send_modify(&self, f: impl FnOnce(&mut S)) {
        self.tx.send_modify(f);
    }

    pub fn mark_changed(&mut self) {
        self.tx.send_replace(self.latched.clone());
    }

    pub fn change_detector(&self) -> ValStateChangeDetector<S> {
        ValStateChangeDetector {
            rx: self.tx.subscribe(),
        }
    }

    pub fn handle(&self) -> ValStateHandle<S> {
        ValStateHandle {
            latched: self.latched.clone(),
            tx: self.tx.clone(),
        }
    }

    pub fn with_mut_for_ui(
        &mut self,
        ui: &mut Ui,
        f: impl FnOnce(&mut Ui, &mut S) -> Response,
    ) -> Response {
        let resp = f(ui, self.value_mut_untracked());
        if resp.changed() {
            self.mark_changed()
        }

        resp
    }
}

pub struct ValStateChangeDetector<S> {
    rx: watch::Receiver<S>,
}

impl<S> Clone for ValStateChangeDetector<S> {
    fn clone(&self) -> Self {
        Self {
            rx: self.rx.clone(),
        }
    }
}
impl<S: 'static + Send + Sync> ChangeDetector for ValStateChangeDetector<S> {
    fn wait_for_change(&self) -> Pin<Box<dyn Future<Output = Option<()>> + Send + 'static>> {
        let mut this = self.clone();
        Box::pin(async move { this.rx.changed().await.ok() })
    }
}

#[derive(Clone)]
pub struct ValStateHandle<S> {
    latched: S,
    tx: watch::Sender<S>,
}

impl<S> ValStateHandle<S> {
    pub fn set(&mut self, value: S) {
        self.tx.send_replace(value);
    }

    pub fn value(&self) -> &S {
        &self.latched
    }

    pub fn value_mut(&mut self) -> &mut S {
        &mut self.latched
    }

    pub fn latest_value(&self) -> S
    where
        S: Clone,
    {
        self.tx.borrow().clone()
    }

    pub fn send_value(&self, value: S) {
        let _ = self.tx.send(value);
    }

    pub fn send_update(&self, f: impl FnOnce(&mut S)) {
        self.tx.send_modify(f);
    }

    pub fn maybe_send_update(&self, f: impl FnOnce(&mut S) -> bool) {
        self.tx.send_if_modified(f);
    }
}

impl<S: Send + Sync + 'static> Stateful for ValState<S> {
    type ChangeDetector = ValStateChangeDetector<S>;
    type Handle = ValStateHandle<S>;
}

impl<S: Send + Sync + Clone + 'static> ViewModelLike for ValState<S> {
    fn latch_state(&mut self) {
        self.latch_value()
    }

    fn change_detector_boxed(&self) -> Box<dyn ChangeDetector> {
        Box::new(self.change_detector())
    }
}

impl<S: Send + Sync + Clone + 'static> ViewModel for ValState<S> {
    type Model = ValStateHandle<S>;
    type ChangeDetector = ValStateChangeDetector<S>;

    fn make_model(&self) -> Self::Model {
        self.handle()
    }

    fn change_detector(&self) -> Self::ChangeDetector {
        self.change_detector()
    }
}

impl<T: Send + Sync + Clone + 'static> From<T> for ValState<T> {
    fn from(value: T) -> Self {
        ValState::new(value)
    }
}
