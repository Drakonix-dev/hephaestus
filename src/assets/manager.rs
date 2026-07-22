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
        Asset, AssetError, AssetFailed, AssetLoaded, AssetStatus, SourceFor,
        graph::{Graph, Node},
        loader::{BuildFn, ErasedLoader, Loader, LoaderCell},
        pool::{Pool, Priority},
        registry::{ErasedRegistry, Handle, Registry},
    },
    config::EngineConfig,
    events::EventBus,
};

type ApplyFn =
    Box<dyn FnOnce(&mut dyn ErasedRegistry, &mut EventBus) -> Result<(), AssetError> + Send>;

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

    pub(crate) fn close(self) {
        self.pool.close()
    }

    pub fn load<A: Asset, S: SourceFor<A, Raw: Send> + Send + Sync>(
        &mut self,
        src: S,
        priority: Priority,
    ) -> Result<Handle<A>, AssetError> {
        let registry =
            self.registry
                .get_mut(&TypeId::of::<A>())
                .ok_or(AssetError::UnknownAsset {
                    t: type_name::<A>(),
                })?;

        let key = (TypeId::of::<A>(), TypeId::of::<S>());
        let loader = self
            .loaders
            .get(&key)
            .ok_or(AssetError::UnhandledAsset {
                t: type_name::<A>(),
            })?
            .clone();

        let (id, generation) = registry.insert_pending();
        let handle = Handle::new(id, generation);

        let tx = self.qtx.clone();
        let submit = Box::new(
            move |result: Result<(BuildFn, Option<Vec<Node>>), AssetError>| {
                let apply: ApplyFn = Box::new(move |reg, events| {
                    match result.and_then(|(build, deps)| build(&handle, reg)) {
                        Ok(()) => events.publish(AssetLoaded { handle }),
                        Err(e) => {
                            let reason = e.to_string();
                            reg.failed(handle.id, handle.generation, Box::new(e))?;
                            events.publish(AssetFailed { handle, reason });
                        }
                    }

                    Ok(())
                });

                let _ = tx.send(QueuedAsset {
                    apply,
                    type_id: TypeId::of::<A>(),
                    type_name: type_name::<A>(),
                });
            },
        );

        let src = Arc::new(src);
        self.pool.load(src.clone(), priority, loader, submit);

        Ok(handle)
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

            (job.apply)(reg.as_mut(), events)?;
        }

        Ok(())
    }

    pub fn register<A, S, L>(&mut self, l: L)
    where
        A: Asset,
        S: SourceFor<A, Raw: Send>,
        L: Loader<A, S, Parsed: Send> + Send + Sync + 'static,
    {
        self.registry
            .entry(TypeId::of::<A>())
            .or_insert_with(|| Box::new(Registry::<A, L::Built>::new()));
        self.loaders
            .entry((TypeId::of::<A>(), TypeId::of::<S>()))
            .or_insert_with(|| Arc::new(LoaderCell::<A, S, L>::new(l)));
    }

    pub fn status<A: Asset>(&self, handle: Handle<A>) -> Result<AssetStatus, AssetError> {
        self.registry
            .get(&TypeId::of::<A>())
            .ok_or(AssetError::NotFound {
                id: handle.id,
                t: type_name::<A>().to_string(),
            })?
            .status(handle.id, handle.generation)
    }
}
