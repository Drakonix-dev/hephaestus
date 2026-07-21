pub(crate) mod loader;
mod manager;
pub(crate) mod pool;
pub(crate) mod registry;

pub use {
    loader::Loader as AssetLoader, manager::Manager as AssetManager,
    pool::Priority as AssetPriority, registry::Handle as AssetHandle,
};

use std::error::Error;

use crate::events::Event;

pub trait Asset: 'static {}

pub struct AssetFailed<A: Asset> {
    pub handle: AssetHandle<A>,
    pub reason: String,
}

impl<A: Asset> Event for AssetFailed<A> {}

pub struct AssetLoaded<A: Asset> {
    pub handle: AssetHandle<A>,
}

impl<A: Asset> Event for AssetLoaded<A> {}

pub trait SourceFor<A: Asset>: 'static {
    type Raw;

    fn fetch(&self) -> Result<Self::Raw, AssetError>;
}

pub enum AssetStatus {
    Failed(String),
    Pending,
    Ready,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: String },

    #[error("asset operation failed")]
    Other {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("asset cannot be handled: {t}")]
    UnhandledAsset { t: String },

    #[error("asset type unknown: {t}")]
    UnknownAsset { t: String },
}
