use crate::view_model::{ViewModel, ViewModelLike};
use crate::{ChangeDetector, Stateful};
use egui::{Response, Ui};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::sync::watch;

/// Use this for state where you typically need &mut access and clones are expensive.
#[derive(Clone)]
pub struct RefState<S> {
    latched: Arc<RwLock<S>>,
    tx: watch::Sender<Arc<RwLock<S>>>,
    rx: watch::Receiver<Arc<RwLock<S>>>,
}

impl<S: Default + Send + Sync + 'static> Default for RefState<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

pub struct RefStateMutRef<'a, S> {
    state: RwLockWriteGuard<'a, S>,
    value: Arc<RwLock<S>>,
    changed: Option<bool>,
    tx: watch::Sender<Arc<RwLock<S>>>,
}

impl<S> Drop for RefStateMutRef<'_, S> {
    fn drop(&mut self) {
        if self.changed == Some(true) {
            let _ = self.tx.send(self.value.clone());
        }
    }
}

impl<S> Deref for RefStateMutRef<'_, S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S> DerefMut for RefStateMutRef<'_, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.changed.is_some() {
            self.changed.replace(true);
        }

        &mut self.state
    }
}

impl<S: 'static + Send + Sync> RefState<S> {
    pub fn new(value: S) -> Self {
        let value = Arc::new(RwLock::new(value));
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

    pub fn latest_value(&self) -> Arc<RwLock<S>> {
        self.tx.borrow().clone()
    }

    pub fn value(&self) -> RefStateRef<'_, S> {
        RefStateRef(self.latched.read().unwrap())
    }

    pub fn value_mut(&mut self) -> RefStateMutRef<'_, S> {
        RefStateMutRef {
            value: self.latched.clone(),
            state: self.latched.write().unwrap(),
            changed: Some(false),
            tx: self.tx.clone(),
        }
    }

    pub fn value_mut_untracked(&mut self) -> RefStateMutRef<'_, S> {
        RefStateMutRef {
            value: self.latched.clone(),
            state: self.latched.write().unwrap(),
            changed: None,
            tx: self.tx.clone(),
        }
    }

    pub fn send_value(&self, value: S) {
        let _ = self.tx.send(Arc::new(RwLock::new(value)));
    }

    pub fn send_modify(&self, f: impl FnOnce(&mut S)) {
        self.tx.send_modify(|t| f(&mut t.write().unwrap()));
    }

    pub fn mark_changed(&mut self) {
        self.tx.send_replace(self.latched.clone());
    }

    pub fn change_detector(&self) -> RefStateChangeDetector<S> {
        RefStateChangeDetector {
            rx: self.tx.subscribe(),
        }
    }

    pub fn handle(&self) -> RefStateHandle<S> {
        RefStateHandle {
            latched: self.latched.clone(),
            tx: self.tx.clone(),
        }
    }

    pub fn with_mut_for_ui(
        &mut self,
        ui: &mut Ui,
        f: impl FnOnce(&mut Ui, &mut S) -> Response,
    ) -> Response {
        let resp = f(ui, &mut self.value_mut_untracked());
        if resp.changed() {
            self.mark_changed()
        }

        resp
    }
}

pub struct RefStateChangeDetector<S> {
    rx: watch::Receiver<Arc<RwLock<S>>>,
}

impl<S> Clone for RefStateChangeDetector<S> {
    fn clone(&self) -> Self {
        Self {
            rx: self.rx.clone(),
        }
    }
}
impl<S: 'static + Send + Sync> ChangeDetector for RefStateChangeDetector<S> {
    fn wait_for_change(&self) -> Pin<Box<dyn Future<Output = Option<()>> + Send + 'static>> {
        let mut this = self.clone();
        Box::pin(async move { this.rx.changed().await.ok() })
    }
}

#[derive(Clone)]
pub struct RefStateHandle<S> {
    latched: Arc<RwLock<S>>,
    tx: watch::Sender<Arc<RwLock<S>>>,
}

impl<S> RefStateHandle<S> {
    pub fn set(&mut self, value: S) {
        self.tx.send_replace(Arc::new(RwLock::new(value)));
    }

    pub fn value(&self) -> RefStateHandleRef<'_, S> {
        RefStateHandleRef(self.latched.read().unwrap())
    }

    pub fn value_mut(&mut self) -> RefStateHandleMutRef<'_, S> {
        RefStateHandleMutRef(self.latched.write().unwrap())
    }

    pub fn latest_value(&self) -> Arc<RwLock<S>> {
        self.tx.borrow().clone()
    }

    pub fn send_value(&self, value: S) {
        let _ = self.tx.send(Arc::new(RwLock::new(value)));
    }

    pub fn send_update(&self, f: impl FnOnce(&mut S)) {
        self.tx.send_modify(|t| f(&mut t.write().unwrap()));
    }

    pub fn maybe_send_update(&self, f: impl FnOnce(&mut S) -> bool) {
        self.tx.send_if_modified(|t| f(&mut t.write().unwrap()));
    }
}

pub struct RefStateRef<'a, T>(RwLockReadGuard<'a, T>);

impl<T> Deref for RefStateRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct RefStateHandleRef<'a, T>(RwLockReadGuard<'a, T>);

impl<T> Deref for RefStateHandleRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct RefStateHandleMutRef<'a, T>(RwLockWriteGuard<'a, T>);

impl<T> Deref for RefStateHandleMutRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RefStateHandleMutRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<S: Send + Sync + 'static> Stateful for RefState<S> {
    type ChangeDetector = RefStateChangeDetector<S>;
    type Handle = RefStateHandle<S>;
}

impl<S: Send + Sync + 'static> ViewModelLike for RefState<S> {
    fn latch_state(&mut self) {
        self.latch_value()
    }

    fn change_detector_boxed(&self) -> Box<dyn ChangeDetector> {
        Box::new(self.change_detector())
    }
}

impl<S: Send + Sync + 'static> ViewModel for RefState<S> {
    type Model = RefStateHandle<S>;
    type ChangeDetector = RefStateChangeDetector<S>;

    fn make_model(&self) -> Self::Model {
        self.handle()
    }

    fn change_detector(&self) -> Self::ChangeDetector {
        self.change_detector()
    }
}

impl<T: Send + Sync + 'static> From<T> for RefState<T> {
    fn from(value: T) -> Self {
        RefState::new(value)
    }
}
