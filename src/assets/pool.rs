use std::sync::Arc;

use crate::assets::{
    Asset, AssetError, Handle, SourceFor, loader::ErasedLoader, registry::ErasedRegistry,
};

pub enum Priority {
    Critical,
    Streaming,
    Idle,
}

pub(crate) struct Pool {}

impl Pool {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn load<A: Asset, S: SourceFor<A>>(
        &self,
        handle: Handle<A>,
        src: &S,
        _priority: Priority,
        loader: Arc<dyn ErasedLoader>,
        reg: &mut dyn ErasedRegistry,
    ) -> Result<(), AssetError> {
        let raw = src.fetch()?;
        let build = loader.parse(Box::new(raw));
        (build)(&handle, reg)
    }
}
