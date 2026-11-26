use core::fmt;
use std::{
    any::{Any, TypeId},
    borrow::Cow,
    collections::HashMap,
    error::Error,
    marker::PhantomData,
    sync::Arc,
};

use bytes::Bytes;

use crate::assets::{
    Asset, AssetError, AssetLabel, GameAsset, INVARIANT, Stores, WeakHandle,
};

pub(super) type BuildFn =
    Box<dyn FnOnce() -> Result<Box<dyn Any>, AssetError> + Send>;

pub(super) trait ErasedLoader: Send + Sync + 'static {
    fn parse(
        self: Arc<Self>,
        desc: Box<dyn Any + Send>,
        label: AssetLabel,
        bytes: Option<Bytes>,
        deps: &mut Deps,
    ) -> Result<BuildFn, AssetError>;
}

pub trait Loader<A: Asset>: Send + Sync + 'static {
    type Desc: Descriptor;
    type Error: Error + Send + Sync + 'static;
    type Parsed: Send + 'static;

    fn build(
        &self,
        desc: &Self::Desc,
        parsed: Self::Parsed,
    ) -> Result<A::Built, Self::Error>;
    fn parse(
        &self,
        desc: &Self::Desc,
        bytes: Option<Bytes>,
        deps: &mut Deps,
    ) -> Result<Self::Parsed, Self::Error>;
}

pub(super) struct LoaderCell<A, L>(L, PhantomData<fn() -> A>);

impl<A, L> LoaderCell<A, L> {
    pub(super) fn new(l: L) -> Self {
        Self(l, PhantomData)
    }
}

impl<A, L> ErasedLoader for LoaderCell<A, L>
where
    A: Asset,
    L: Loader<A>,
{
    fn parse(
        self: Arc<Self>,
        desc: Box<dyn Any + Send>,
        label: AssetLabel,
        bytes: Option<Bytes>,
        deps: &mut Deps,
    ) -> Result<BuildFn, AssetError> {
        let desc = desc.downcast::<L::Desc>().expect(INVARIANT);
        let parsed = self.0.parse(&desc, bytes, deps).map_err(|source| {
            AssetError::Parse {
                asset: label.clone(),
                source: Box::new(source),
            }
        })?;

        Ok(Box::new(move || {
            self.0
                .build(&desc, parsed)
                .map(|b| -> Box<dyn Any> { Box::new(b) })
                .map_err(|source| AssetError::Build {
                    asset: label,
                    source: Box::new(source),
                })
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath {
    pub scheme: Cow<'static, str>,
    pub path: Cow<'static, str>,
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme, self.path)
    }
}

impl From<&'static str> for AssetPath {
    fn from(s: &'static str) -> Self {
        match s.split_once("://") {
            Some((scheme, path)) => Self {
                scheme: scheme.into(),
                path: path.into(),
            },
            None => Self {
                scheme: "file".into(),
                path: s.into(),
            },
        }
    }
}

pub enum AssetData {
    Inline(Bytes),
    Path(AssetPath),
}

pub trait Descriptor: Send + 'static {
    fn take_external(&mut self) -> Option<AssetData> {
        None
    }

    fn label(&self) -> Option<AssetLabel> {
        None
    }
}

pub struct Deps {
    keys: Vec<(ErasedKey, DepKind)>,
}

impl Deps {
    pub fn optional<A: Asset>(&mut self, h: impl Into<WeakHandle<A>>) {
        self.keys.push((ErasedKey::new(h), DepKind::Optional))
    }

    pub fn require<A: Asset>(&mut self, h: impl Into<WeakHandle<A>>) {
        self.keys.push((ErasedKey::new(h), DepKind::Required))
    }
}

pub(super) enum DepKind {
    Optional,
    Required,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ErasedKey {
    pub(super) id: usize,
    pub(super) generation: u64,
    pub(super) type_id: TypeId,
}

impl ErasedKey {
    fn new<A: Asset>(h: impl Into<WeakHandle<A>>) -> Self {
        let key = h.into().key();
        Self {
            id: key.id,
            generation: key.generation,
            type_id: key.type_id,
        }
    }
}

pub(super) struct Parked {
    pub(super) apply: Box<dyn FnOnce()>,
    pub(super) remaining: usize,
}

#[derive(Default)]
pub(super) struct Dependencies {
    pub(super) dependendents: HashMap<ErasedKey, Vec<ErasedKey>>,
    pub(super) parked: HashMap<ErasedKey, Parked>,
}

pub struct BuildCtx<'a> {
    declared: &'a [(ErasedKey, DepKind)],
    stores: &'a Stores,
}

impl BuildCtx<'_> {
    pub fn get<A: GameAsset>(&self, h: impl Into<WeakHandle<A>>) -> &A::Built {
        self.get_built(h)
    }

    pub(crate) fn get_built<A: Asset>(
        &self,
        h: impl Into<WeakHandle<A>>,
    ) -> &A::Built {
        todo!()
    }

    pub(crate) fn get_built_optional<A: Asset>(
        &self,
        h: impl Into<WeakHandle<A>>,
    ) -> Option<&A::Built> {
        todo!()
    }

    pub fn get_optional<A: GameAsset>(
        &self,
        h: impl Into<WeakHandle<A>>,
    ) -> Option<&A::Built> {
        self.get_built_optional(h)
    }
}
