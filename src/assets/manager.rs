use std::{
    any::{TypeId, type_name},
    collections::HashMap,
};

use crate::assets::{
    Asset, AssetError, AssetStatus, Handle, Loader, SourceFor,
    loader::{ErasedLoader, LoaderCell},
    registry::{ErasedRegistry, Registry},
};

pub struct Manager {
    loaders: HashMap<TypeId, Box<dyn ErasedLoader>>,
    registry: HashMap<TypeId, Box<dyn ErasedRegistry>>,
}

impl Manager {
    pub(crate) fn new() -> Self {
        Self {
            loaders: HashMap::new(),
            registry: HashMap::new(),
        }
    }

    pub fn load<A: Asset, S: SourceFor<A> + Send>(
        &mut self,
        src: S,
    ) -> Result<Handle<A>, AssetError> {
        let (id, generation) = self
            .registry
            .get_mut(&TypeId::of::<A>())
            .ok_or(AssetError::UnknownAsset {
                t: type_name::<A>().to_string(),
            })?
            .insert_pending();

        let _ = self
            .loaders
            .get(&TypeId::of::<A>())
            .ok_or(AssetError::UnhandledAsset {
                t: type_name::<A>().to_string(),
            })?
            .parse(Box::new(src.fetch()));

        Ok(Handle::new(id, generation))
    }

    pub fn register<A, S, L>(&mut self, l: L)
    where
        A: Asset,
        S: SourceFor<A> + Send,
        L: Loader<A, S> + Send + Sync + 'static,
    {
        self.registry
            .entry(TypeId::of::<A>())
            .or_insert_with(|| Box::new(Registry::<A, L::Built>::new()));
        self.loaders
            .entry(TypeId::of::<A>())
            .or_insert_with(|| Box::new(LoaderCell::<A, S, L>::new(l)));
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
