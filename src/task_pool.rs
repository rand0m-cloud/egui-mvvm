use std::sync::Mutex;
use tokio::task::JoinSet;

#[derive(Default, Debug)]
pub struct TaskPool {
    join_set: Mutex<JoinSet<()>>,
}

impl TaskPool {
    pub fn new() -> Self {
        Self {
            join_set: Default::default(),
        }
    }

    pub fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        self.join_set.lock().unwrap().spawn(task);
    }

    pub fn spawn_local(&self, task: impl Future<Output = ()> + 'static) {
        self.join_set.lock().unwrap().spawn_local(task);
    }
}
