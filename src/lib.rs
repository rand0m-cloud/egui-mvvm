use std::pin::Pin;

pub mod state;
pub mod task_pool;
pub mod view_model;

pub trait ChangeDetector: Sync + Send + 'static {
    fn wait_for_change(&self) -> Pin<Box<dyn Future<Output = Option<()>> + Send + 'static>>;
}
