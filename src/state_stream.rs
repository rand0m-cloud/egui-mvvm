use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::watch;

pub struct StateStream<S> {
    last: Arc<S>,
    tx: watch::Sender<Arc<S>>,
    rx: watch::Receiver<Arc<S>>,
}

impl<S> StateStream<S> {
    pub fn new(initial_state: S) -> Self {
        let initial_state = Arc::new(initial_state);
        let (tx, rx) = watch::channel(initial_state.clone());
        Self {
            last: initial_state,
            tx,
            rx,
        }
    }

    pub fn update(&self, f: impl FnOnce(&S) -> S) {
        self.tx.send_modify(|state| {
            *state = Arc::new(f(&state));
        });
    }

    pub fn maybe_update(&self, f: impl FnOnce(&S) -> Option<S>) {
        self.tx.send_if_modified(|state| {
            if let Some(new) = f(&state) {
                *state = Arc::new(new);
                true
            } else {
                false
            }
        });
    }

    pub fn value(&self) -> Arc<S> {
        self.last.clone()
    }

    pub fn latest_value(&self) -> Arc<S> {
        self.rx.borrow().clone()
    }

    pub fn latch_value(&mut self) {
        self.last = self.rx.borrow_and_update().clone();
    }

    pub fn set_value(&self, value: S) {
        self.tx.send_replace(Arc::new(value));
    }

    pub fn has_changed(&self) -> bool {
        self.rx.has_changed().unwrap_or(false)
    }

    pub fn change_detector(&self) -> StateStreamChangeDetector<S> {
        StateStreamChangeDetector(self.rx.clone())
    }

    pub fn handle(&self) -> StateStreamHandle<S> {
        StateStreamHandle {
            rx: self.rx.clone(),
            tx: self.tx.clone(),
        }
    }
}

pub struct StateStreamChangeDetector<S>(watch::Receiver<Arc<S>>);

impl<S: Send + Sync + 'static> ChangeDetector for StateStreamChangeDetector<S> {
    async fn wait_for_change(&mut self) -> Option<()> {
        self.0.changed().await.ok()
    }
}

pub trait ChangeDetector: Sync + Send + 'static {
    fn wait_for_change(&mut self) -> impl Future<Output = Option<()>> + Send;
}

#[derive(Clone)]
pub struct StateStreamHandle<S> {
    tx: watch::Sender<Arc<S>>,
    rx: watch::Receiver<Arc<S>>,
}

impl<S> StateStreamHandle<S> {
    pub fn update(&self, f: impl FnOnce(&S) -> S) {
        self.tx.send_modify(|state| {
            *state = Arc::new(f(&state));
        });
    }

    pub fn maybe_update(&self, f: impl FnOnce(&S) -> Option<S>) {
        self.tx.send_if_modified(|state| {
            if let Some(new) = f(&state) {
                *state = Arc::new(new);
                true
            } else {
                false
            }
        });
    }

    pub fn latest_value(&self) -> Arc<S> {
        self.rx.borrow().clone()
    }

    pub fn set_value(&self, value: S) {
        self.tx.send_replace(Arc::new(value));
    }
}
