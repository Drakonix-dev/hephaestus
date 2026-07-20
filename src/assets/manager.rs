use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    sync::Arc,
};

use crate::{
    assets::{
        Asset, AssetError, AssetStatus, Handle, Loader, Priority, SourceFor,
        loader::{ErasedLoader, LoaderCell},
        pool::Pool,
        registry::{ErasedRegistry, Registry},
    },
    config::EngineConfig,
    events::EventBus,
};

pub struct Manager {
    loaders: HashMap<(TypeId, TypeId), Arc<dyn ErasedLoader>>,
    pool: Pool,
    registry: HashMap<TypeId, Box<dyn ErasedRegistry>>,
}

impl Manager {
    pub(crate) fn new(cfg: &EngineConfig, events: Arc<EventBus>) -> Self {
        Self {
            loaders: HashMap::new(),
            pool: Pool::new(cfg, events),
            registry: HashMap::new(),
        }
    }

    pub fn load<A: Asset, S: SourceFor<A> + Send>(
        &mut self,
        src: S,
        priority: Priority,
    ) -> Result<Handle<A>, AssetError> {
        let registry =
            self.registry
                .get_mut(&TypeId::of::<A>())
                .ok_or(AssetError::UnknownAsset {
                    t: type_name::<A>().to_string(),
                })?;

        let key = (TypeId::of::<A>(), TypeId::of::<S>());
        let loader = self
            .loaders
            .get(&key)
            .ok_or(AssetError::UnhandledAsset {
                t: type_name::<A>().to_string(),
            })?
            .clone();

        let (id, generation) = registry.insert_pending();
        let handle = Handle::new(id, generation);
        self.pool
            .load(handle, src, priority, loader, registry.as_mut());

        Ok(handle)
    }

    pub fn register<A, S, L>(&mut self, l: L)
    where
        A: Asset,
        S: SourceFor<A>,
        L: Loader<A, S> + 'static,
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
