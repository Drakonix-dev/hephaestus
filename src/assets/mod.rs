mod manager;

pub(crate) mod loader;
pub(crate) mod pool;
pub(crate) mod registry;

pub mod events;
pub mod graph;

use std::{any::type_name, error::Error, io, marker::PhantomData};

pub use {loader::Loader, manager::Manager};

pub(crate) const INVARIANT: &str = "asset type-erasure invariant violated";

pub trait Asset: 'static {}

pub trait SourceFor<A: Asset>: Send + Sync + 'static {
    type Raw: Send;

    fn fetch(&self) -> Result<Self::Raw, AssetError>;
}

pub(crate) trait BuiltAs: Asset {
    type Built: 'static;
}

pub trait GameAsset: Asset {
    type Built: 'static;
}

impl<T: GameAsset> BuiltAs for T {
    type Built = <T as GameAsset>::Built;
}

pub enum AssetStatus {
    Failed(String),
    Pending,
    Ready,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub(crate) generation: u64,
    pub(crate) id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub(crate) fn new(id: usize, generation: u64) -> Self {
        Self {
            generation,
            id,
            _marker: PhantomData,
        }
    }

    pub fn not_found(self) -> AssetError {
        AssetError::NotFound {
            id: self.id,
            t: type_name::<T>(),
        }
    }

    pub fn not_ready(self) -> AssetError {
        AssetError::NotReady {
            id: self.id,
            t: type_name::<T>(),
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation.clone(),
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}

pub enum Priority {
    Critical,
    Streaming,
    Idle,
}

const TOTAL_PRIORITIES: usize = Priority::Idle as usize + 1;

pub struct Dependency<T> {
    pub(crate) idx: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Dependency<T> {
    pub(crate) fn new(idx: usize) -> Self {
        Self {
            idx,
            _marker: PhantomData,
        }
    }

    pub fn not_found(self) -> AssetError {
        AssetError::NotFound {
            id: self.idx,
            t: type_name::<T>(),
        }
    }
}

pub enum DependencyKind {
    Optional,
    Required,
}

pub struct Resolved<'a, T> {
    data: Option<&'a T>,
    id: usize,
}

impl<'a, T> Resolved<'a, T> {
    pub(crate) fn new(id: usize, data: Option<&'a T>) -> Self {
        Self { data, id }
    }

    pub fn required(self) -> Result<&'a T, AssetError> {
        self.data.ok_or(AssetError::NotReady {
            id: self.id,
            t: type_name::<T>(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("bad asset: {reason}")]
    BadAsset { reason: &'static str },

    #[error(transparent)]
    IO(#[from] io::Error),

    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: &'static str },

    #[error("asset not ready: {t}x{id}")]
    NotReady { id: usize, t: &'static str },

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
