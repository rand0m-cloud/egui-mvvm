use crate::ChangeDetector;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use tokio::sync::watch;

/// Use this for state where you typically need &mut access. Updating state uses a clone of [`S`].
pub struct State<S> {
    latched: S,
    tx: watch::Sender<S>,
    rx: watch::Receiver<S>,
}

impl<S: Default + Clone + Send + Sync + 'static> Default for State<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

pub struct StateMutRef<'a, S> {
    state: &'a mut S,
    mark_on_death: bool,
    tx: watch::Sender<S>,
}

impl<S> Drop for StateMutRef<'_, S> {
    fn drop(&mut self) {
        if self.mark_on_death {
            self.tx.send_modify(|_| {});
        }
    }
}

impl<S> Deref for StateMutRef<'_, S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<S> DerefMut for StateMutRef<'_, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}

impl<S: 'static + Send + Sync + Clone> State<S> {
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

    pub fn value_mut(&mut self) -> StateMutRef<'_, S> {
        StateMutRef {
            state: &mut self.latched,
            mark_on_death: true,
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

    pub fn change_detector(&self) -> StateChangeDetector<S> {
        StateChangeDetector {
            rx: self.tx.subscribe(),
        }
    }

    pub fn handle(&self) -> StateHandle<S> {
        StateHandle {
            latched: self.latched.clone(),
            tx: self.tx.clone(),
        }
    }
}

pub struct StateChangeDetector<S> {
    rx: watch::Receiver<S>,
}

impl<S> Clone for StateChangeDetector<S> {
    fn clone(&self) -> Self {
        Self {
            rx: self.rx.clone(),
        }
    }
}
impl<S: 'static + Send + Sync> ChangeDetector for StateChangeDetector<S> {
    fn wait_for_change(&self) -> Pin<Box<dyn Future<Output = Option<()>> + Send + 'static>> {
        let mut this = self.clone();
        Box::pin(async move { this.rx.changed().await.ok() })
    }
}

pub struct StateHandle<S> {
    latched: S,
    tx: watch::Sender<S>,
}

impl<S> StateHandle<S> {
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
