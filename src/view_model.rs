use crate::task_pool::{TaskHandle, TaskPool};
use crate::ChangeDetector;
use egui::{Id, UiBuilder};
use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak};
use tokio::sync::watch;

pub trait ViewModel: ViewModelLike {
    type Model: 'static;
    type ChangeDetector: ChangeDetector;

    fn make_model(&self) -> Self::Model;
    fn change_detector(&self) -> Self::ChangeDetector;

    fn spawn<F>(&self, f: impl FnOnce(Self::Model) -> F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static,
        Self: ViewModelTaskPool,
    {
        self.task_pool().spawn(f(self.make_model()))
    }

    fn spawn_local<F>(&self, f: impl FnOnce(Self::Model) -> F) -> TaskHandle
    where
        F: Future<Output = ()> + 'static,
        Self: ViewModelTaskPool,
    {
        self.task_pool().spawn_local(f(self.make_model()))
    }
}

pub trait ViewModelTaskPool {
    fn task_pool(&self) -> TaskPool;
}

pub trait ViewModelLike: Any + Send + Sync + 'static {
    fn latch_state(&mut self);
    fn change_detector_boxed(&self) -> Box<dyn ChangeDetector>;
}

#[derive(Clone)]
pub struct ViewModelsChangeDetector {
    rx: watch::Receiver<Vec<Weak<RwLock<dyn ViewModelLike>>>>,
}

impl ChangeDetector for ViewModelsChangeDetector {
    fn wait_for_change(&self) -> Pin<Box<dyn Future<Output = Option<()>> + Send>> {
        let mut this = self.clone();
        Box::pin(async move {
            // Create a list of the wait_for_change futures for all view models.
            let list = this.rx.borrow_and_update().clone();

            let list = list
                .iter()
                .filter_map(|weak| weak.upgrade())
                .map(|vm| vm.read().unwrap().change_detector_boxed().wait_for_change())
                .collect::<Vec<_>>();

            if list.is_empty() {
                this.rx.changed().await.ok()
            } else {
                tokio::select! {
                    res = this.rx.changed() => {
                        res.ok()
                    }
                    (res, _, _) = futures::future::select_all(list) => {
                        res
                    }
                }
            }
        })
    }
}

#[derive(Clone, Default)]
pub struct ViewModels(Arc<Mutex<ViewModelsInner>>);

impl ViewModels {
    pub fn change_detector(&self) -> ViewModelsChangeDetector {
        ViewModelsChangeDetector {
            rx: self.0.lock().unwrap().tx.subscribe(),
        }
    }

    pub fn latch_values(&mut self) {
        self.0.lock().unwrap().view_models.retain(|vm| {
            let mut keep = false;

            if let Some(vm) = vm.upgrade() {
                if let Ok(mut vm) = vm.write() {
                    keep = true;
                    vm.latch_state();
                }
            }

            keep
        })
    }

    pub fn add<T: ViewModel>(&self, vm: &ViewModelHandle<T>) {
        let mut this = self.0.lock().unwrap();
        this.tx.send_modify(|v| {
            v.push(Arc::downgrade(&vm.0) as Weak<_>);
        });

        let new = this.tx.borrow().clone();
        this.view_models = new;
    }
}

#[derive(Default)]
pub struct ViewModelsInner {
    pub view_models: Vec<Weak<RwLock<dyn ViewModelLike>>>,
    tx: watch::Sender<Vec<Weak<RwLock<dyn ViewModelLike>>>>,
}

pub trait EguiViewModelExt {
    fn fetch_model<V: ViewModel + Default>(self) -> ViewModelHandle<V>;
    fn fetch_model_or_insert<V: ViewModel, F: FnOnce() -> V>(self, f: F) -> ViewModelHandle<V>;
}

impl EguiViewModelExt for &mut egui::Ui {
    fn fetch_model<V: ViewModel + Default>(self) -> ViewModelHandle<V> {
        self.fetch_model_or_insert(|| Default::default())
    }

    fn fetch_model_or_insert<V: ViewModel, F: FnOnce() -> V>(self, f: F) -> ViewModelHandle<V> {
        let id = self.allocate_new_ui(UiBuilder::new(), |ui| ui.id()).inner;
        let mut inserted = false;
        let vm = self.memory_mut(|mem| {
            let vm = mem
                .data
                .get_temp_mut_or_insert_with::<ViewModelHandle<V>>(id, || {
                    inserted = true;
                    ViewModelHandle(Arc::new(RwLock::new(f())))
                })
                .clone();

            vm
        });

        if inserted {
            let vms = self.memory_mut(|mem| mem.view_models());
            vms.add(&vm);
        }

        vm
    }
}

pub trait EguiViewModelsExt {
    fn view_models(self) -> ViewModels;
}

impl EguiViewModelsExt for &mut egui::Memory {
    fn view_models(self) -> ViewModels {
        self.data
            .get_temp_mut_or_default::<ViewModels>(Id::NULL)
            .clone()
    }
}

#[derive(Default)]
pub struct ViewModelHandle<V>(Arc<RwLock<V>>);

pub struct ViewModelRef<'a, V>(RwLockReadGuard<'a, V>, ViewModelHandle<V>);
pub struct ViewModelMutRef<'a, V>(RwLockWriteGuard<'a, V>, ViewModelHandle<V>);

impl<V> ViewModelRef<'_, V> {
    pub fn handle(&self) -> &ViewModelHandle<V> {
        &self.1
    }
}

impl<V> ViewModelMutRef<'_, V> {
    pub fn handle(&self) -> &ViewModelHandle<V> {
        &self.1
    }
}

impl<V> ViewModelHandle<V> {
    pub fn get(&self) -> ViewModelRef<V> {
        ViewModelRef(self.0.read().unwrap(), self.clone())
    }

    pub fn get_mut(&self) -> ViewModelMutRef<V> {
        ViewModelMutRef(self.0.write().unwrap(), self.clone())
    }
}

impl<V> Clone for ViewModelHandle<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V> Deref for ViewModelRef<'_, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Deref for ViewModelMutRef<'_, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<V> DerefMut for ViewModelMutRef<'_, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub async fn request_repaint_on_change(ctx: egui::Context) -> ! {
    let view_models = ctx.memory_mut(|mem| mem.view_models());

    // Send Repaint Requests when the ViewModel changes.
    let mut change_detector = view_models.change_detector();
    let ctx = ctx.clone();
    loop {
        if change_detector.wait_for_change().await.is_none() {
            ctx.memory_mut(|mem| {
                change_detector = mem.view_models().change_detector();
            })
        }

        ctx.request_repaint();
    }
}
