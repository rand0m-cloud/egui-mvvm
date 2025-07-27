use egui::Ui;
use std::sync::{Arc, Mutex};
use tokio::task::{AbortHandle, JoinSet};

#[derive(Default, Debug, Clone)]
pub struct TaskPool {
    join_set: Arc<Mutex<JoinSet<()>>>,
}

#[derive(Clone)]
pub struct TaskHandle {
    handle: AbortHandle,
}

impl TaskPool {
    pub fn new() -> Self {
        Self {
            join_set: Default::default(),
        }
    }

    pub fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) -> TaskHandle {
        TaskHandle {
            handle: self.join_set.lock().unwrap().spawn(task),
        }
    }

    pub fn spawn_local(&self, task: impl Future<Output = ()> + 'static) -> TaskHandle {
        TaskHandle {
            handle: self.join_set.lock().unwrap().spawn_local(task),
        }
    }
}

impl TaskHandle {
    pub fn abort(&self) {
        self.handle.abort()
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

pub trait EguiLocalTaskPool {
    fn local_task_pool(&mut self) -> TaskPool;
}

impl EguiLocalTaskPool for &mut Ui {
    fn local_task_pool(&mut self) -> TaskPool {
        let id = self.scope(|ui| ui.id()).inner;
        self.memory_mut(|mem| mem.data.get_temp_mut_or_default::<TaskPool>(id).clone())
    }
}
