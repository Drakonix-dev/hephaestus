mod manager;

pub(crate) mod loader;
pub(crate) mod pool;
pub(crate) mod registry;

pub mod graph;

use std::{error::Error, io};

use crate::events::Event;

pub use {
    loader::Loader as AssetLoader, manager::Manager as AssetManager,
    pool::Priority as AssetPriority, registry::Handle as AssetHandle,
};

pub(crate) const INVARIANT: &str = "asset type-erasure invariant violated";

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

pub trait SourceFor<A: Asset>: Send + Sync + 'static {
    type Raw: Send;

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
    #[error("bad asset: {reason}")]
    BadAsset { reason: &'static str },

    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: String },

    #[error("operation failed")]
    OperationFailed {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("asset cannot be handled: {t}")]
    UnhandledAsset { t: &'static str },

    #[error("asset type unknown: {t}")]
    UnknownAsset { t: &'static str },
}
