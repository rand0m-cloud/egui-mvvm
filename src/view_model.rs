use crate::state_stream::{ChangeDetector, StateStream};
use crate::task_pool::TaskPool;

pub trait ViewModel {
    type Model: 'static;

    fn task_pool(&self) -> &TaskPool;
    fn latch_state(&mut self);
    fn has_changed(&self) -> bool;
    fn make_model(&self) -> Self::Model;
    fn change_detector(&self) -> impl ChangeDetector + 'static;

    fn spawn<F>(&self, f: impl FnOnce(Self::Model) -> F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.task_pool().spawn(f(self.make_model()));
    }

    fn spawn_local<F>(&self, f: impl FnOnce(Self::Model) -> F)
    where
        F: Future<Output = ()> + 'static,
    {
        self.task_pool().spawn_local(f(self.make_model()))
    }
    type ChangeDetector;
}
