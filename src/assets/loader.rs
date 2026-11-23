use core::fmt;
use std::{
    any::Any, borrow::Cow, error::Error, marker::PhantomData, sync::Arc,
};

use bytes::Bytes;

use crate::assets::{
    Asset, AssetError, AssetLabel, ErasedStore, Handle, INVARIANT, SlotState,
    Store,
};

pub(super) type BuildFn = Box<
    dyn FnOnce(&dyn Any, &mut dyn ErasedStore) -> Result<(), AssetError> + Send,
>;

pub(super) trait ErasedLoader: Send + Sync + 'static {
    fn parse(
        self: Arc<Self>,
        desc: Box<dyn Any + Send>,
        label: AssetLabel,
        bytes: Option<Bytes>,
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
    ) -> Result<BuildFn, AssetError> {
        let desc = desc.downcast::<L::Desc>().expect(INVARIANT);
        let parsed =
            self.0
                .parse(&desc, bytes)
                .map_err(|source| AssetError::Parse {
                    asset: label.clone(),
                    source: Box::new(source),
                })?;

        Ok(Box::new(move |handle, store| {
            let h = handle.downcast_ref::<Handle<A>>().expect(INVARIANT);
            let built = self.0.build(&desc, parsed).map_err(|source| {
                AssetError::Build {
                    asset: label,
                    source: Box::new(source),
                }
            })?;

            store
                .as_any_mut()
                .downcast_mut::<Store<A>>()
                .expect(INVARIANT)
                .update(*h, SlotState::Ready(built));

            Ok(())
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
