use crate::ChangeDetector;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::watch;

/// Use this for state where you typically need &mut access and clones are expensive.
#[derive(Clone)]
pub struct RefState<S> {
    latched: Arc<Mutex<S>>,
    tx: watch::Sender<Arc<Mutex<S>>>,
    rx: watch::Receiver<Arc<Mutex<S>>>,
}

impl<S: Default + Send + Sync + 'static> Default for RefState<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

pub struct RefStateMutRef<'a, S> {
    state: MutexGuard<'a, S>,
    value: Arc<Mutex<S>>,
    changed: Option<bool>,
    tx: watch::Sender<Arc<Mutex<S>>>,
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
        let value = Arc::new(Mutex::new(value));
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

    pub fn latest_value(&self) -> Arc<Mutex<S>> {
        self.tx.borrow().clone()
    }

    pub fn value(&self) -> RefStateRef<'_, S> {
        RefStateRef(self.latched.lock().unwrap())
    }

    pub fn value_mut(&mut self) -> RefStateMutRef<'_, S> {
        RefStateMutRef {
            value: self.latched.clone(),
            state: self.latched.lock().unwrap(),
            changed: Some(false),
            tx: self.tx.clone(),
        }
    }

    pub fn value_mut_untracked(&mut self) -> RefStateMutRef<'_, S> {
        RefStateMutRef {
            value: self.latched.clone(),
            state: self.latched.lock().unwrap(),
            changed: None,
            tx: self.tx.clone(),
        }
    }

    pub fn send_value(&self, value: S) {
        let _ = self.tx.send(Arc::new(Mutex::new(value)));
    }

    pub fn send_modify(&self, f: impl FnOnce(&mut S)) {
        self.tx.send_modify(|t| f(&mut t.lock().unwrap()));
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
}

pub struct RefStateChangeDetector<S> {
    rx: watch::Receiver<Arc<Mutex<S>>>,
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

pub struct RefStateHandle<S> {
    latched: Arc<Mutex<S>>,
    tx: watch::Sender<Arc<Mutex<S>>>,
}

impl<S> RefStateHandle<S> {
    pub fn set(&mut self, value: S) {
        self.tx.send_replace(Arc::new(Mutex::new(value)));
    }

    pub fn value(&self) -> RefStateHandleRef<'_, S> {
        RefStateHandleRef(self.latched.lock().unwrap())
    }

    pub fn value_mut(&mut self) -> RefStateHandleMutRef<'_, S> {
        RefStateHandleMutRef(self.latched.lock().unwrap())
    }

    pub fn latest_value(&self) -> Arc<Mutex<S>> {
        self.tx.borrow().clone()
    }

    pub fn send_value(&self, value: S) {
        let _ = self.tx.send(Arc::new(Mutex::new(value)));
    }

    pub fn send_update(&self, f: impl FnOnce(&mut S)) {
        self.tx.send_modify(|t| f(&mut t.lock().unwrap()));
    }

    pub fn maybe_send_update(&self, f: impl FnOnce(&mut S) -> bool) {
        self.tx.send_if_modified(|t| f(&mut t.lock().unwrap()));
    }
}

pub struct RefStateRef<'a, T>(MutexGuard<'a, T>);

impl<'a, T> Deref for RefStateRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct RefStateHandleRef<'a, T>(MutexGuard<'a, T>);

impl<'a, T> Deref for RefStateHandleRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct RefStateHandleMutRef<'a, T>(MutexGuard<'a, T>);

impl<'a, T> Deref for RefStateHandleMutRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T> DerefMut for RefStateHandleMutRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
