use std::pin::Pin;

pub mod hooks;
pub mod ref_state;
pub mod task_pool;
pub mod val_state;
pub mod view_model;

pub trait ChangeDetector: Sync + Send + 'static {
    fn wait_for_change(&self) -> Pin<Box<dyn Future<Output = Option<()>> + Send + 'static>>;
}

pub trait Stateful {
    type ChangeDetector: ChangeDetector;
    type Handle;
}
