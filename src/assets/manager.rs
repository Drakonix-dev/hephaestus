use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};

use crate::{
    assets::{
        AssetError, AssetStatus, BuiltAs, Handle, Priority, SourceFor,
        events::{AssetFailed, AssetLoaded},
        graph::{Deps, Fetch, Graph},
        loader::{BuildFn, ErasedLoader, Loader, LoaderCell},
        pool::{Pool, SubmitFn},
        registry::{ErasedRegistry, Registry},
    },
    config::EngineConfig,
    events::EventBus,
};

type ApplyFn = Box<
    dyn FnOnce(&mut Graph, &mut dyn ErasedRegistry, &mut EventBus) -> Result<(), AssetError> + Send,
>;

struct QueuedAsset {
    apply: ApplyFn,
    type_id: TypeId,
    type_name: &'static str,
}

pub struct Manager {
    dependencies: Graph,
    loaders: HashMap<(TypeId, TypeId), Arc<dyn ErasedLoader>>,
    pool: Pool,
    registry: HashMap<TypeId, Box<dyn ErasedRegistry>>,
    qrx: Receiver<QueuedAsset>,
    qtx: Sender<QueuedAsset>,
}

impl Manager {
    pub(crate) fn new(cfg: &EngineConfig) -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            dependencies: Graph::new(),
            loaders: HashMap::new(),
            pool: Pool::new(cfg),
            registry: HashMap::new(),
            qrx: rx,
            qtx: tx,
        }
    }

    pub fn load<B: BuiltAs, S: SourceFor<B>>(
        &mut self,
        src: S,
        priority: Priority,
    ) -> Result<Handle<B>, AssetError> {
        let registry =
            self.registry
                .get_mut(&TypeId::of::<B>())
                .ok_or(AssetError::UnknownAsset {
                    t: type_name::<B>(),
                })?;

        let key = (TypeId::of::<B>(), TypeId::of::<S>());
        let loader = self
            .loaders
            .get(&key)
            .ok_or(AssetError::UnhandledAsset {
                t: type_name::<B>(),
            })?
            .clone();

        let (id, generation) = registry.insert_pending();
        let handle = Handle::new(id, generation);
        let submit = self.prepare_submit_fn(handle);

        let src = Arc::new(src);
        self.pool.load(src.clone(), priority, loader, submit);

        Ok(handle)
    }

    pub fn register<B, S, L>(&mut self, l: L)
    where
        B: BuiltAs,
        S: SourceFor<B>,
        L: Loader<B, S>,
    {
        self.registry
            .entry(TypeId::of::<B>())
            .or_insert_with(|| Box::new(Registry::<B, B::Built>::new()));
        self.loaders
            .entry((TypeId::of::<B>(), TypeId::of::<S>()))
            .or_insert_with(|| Arc::new(LoaderCell::<B, S, L>::new(l)));
    }

    pub fn status<B: BuiltAs>(&self, handle: Handle<B>) -> Result<AssetStatus, AssetError> {
        self.registry
            .get(&TypeId::of::<B>())
            .ok_or(handle.not_found())?
            .status(handle.id, handle.generation)
    }

    // ------------------------------------------------------------------------

    pub(crate) fn close(self) {
        self.pool.close()
    }

    pub(crate) fn process_queued_assets(
        &mut self,
        events: &mut EventBus,
    ) -> Result<(), AssetError> {
        while let Ok(job) = self.qrx.try_recv() {
            let reg = self
                .registry
                .get_mut(&job.type_id)
                .ok_or(AssetError::UnknownAsset { t: job.type_name })?;

            (job.apply)(&mut self.dependencies, reg.as_mut(), events)?;
        }

        Ok(())
    }

    // ------------------------------------------------------------------------

    fn prepare_apply_fn<B: BuiltAs>(
        &self,
        handle: Handle<B>,
    ) -> Box<dyn FnOnce(Result<(Deps, BuildFn), AssetError>) -> ApplyFn + Send> {
        Box::new(move |result| {
            Box::new(move |graph, reg, events| {
                let mut fetch = Fetch::new();
                let result = result.and_then(|(deps, build)| {
                    deps.add_to_graph(graph);
                    fetch.get_dependencies(&deps, reg);
                    build(&handle, reg, &fetch)
                });

                match result {
                    Ok(()) => events.publish(AssetLoaded { handle }),
                    Err(e) => {
                        let reason = e.to_string();
                        reg.failed(handle.id, handle.generation, Box::new(e))?;
                        events.publish(AssetFailed { handle, reason });
                    }
                }

                Ok(())
            })
        })
    }

    fn prepare_submit_fn<B: BuiltAs>(&self, handle: Handle<B>) -> SubmitFn {
        let tx = self.qtx.clone();
        let get_apply = self.prepare_apply_fn(handle);

        Box::new(move |result: Result<(Deps, BuildFn), AssetError>| {
            let _ = tx.send(QueuedAsset {
                apply: get_apply(result),
                type_id: TypeId::of::<B>(),
                type_name: type_name::<B>(),
            });
        })
    }
}
