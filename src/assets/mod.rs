pub use {
    loader::Loader as AssetLoader, manager::Manager as AssetManager,
    pool::Priority as AssetPriority, registry::Handle as AssetHandle,
};

pub(crate) mod loader;
pub(crate) mod pool;
pub(crate) mod registry;

mod manager;

use std::error::Error;

pub trait Asset: 'static {}

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
