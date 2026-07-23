use std::{any::Any, marker::PhantomData, sync::Arc};

use crate::assets::{
    Asset, AssetError, INVARIANT, SourceFor,
    registry::{ErasedRegistry, Handle, Registry, SlotState},
};

pub(crate) type BuildFn =
    Box<dyn FnOnce(&(dyn Any), &mut (dyn ErasedRegistry)) -> Result<(), AssetError> + Send>;

pub(crate) trait ErasedLoader: Send + Sync {
    fn parse<'a>(
        self: Arc<Self>,
        src: Arc<dyn Any + Send + Sync>,
        raw: Box<dyn Any>,
    ) -> Result<BuildFn, AssetError>;
}

pub(crate) struct LoaderCell<A, S, L>(L, PhantomData<fn() -> (A, S)>);

impl<A, S, L> LoaderCell<A, S, L> {
    pub(crate) fn new(l: L) -> Self {
        Self(l, PhantomData)
    }
}

impl<A, S, L> ErasedLoader for LoaderCell<A, S, L>
where
    A: Asset,
    S: SourceFor<A>,
    L: Loader<A, S>,
{
    fn parse(
        self: Arc<Self>,
        src: Arc<dyn Any + Send + Sync>,
        raw: Box<dyn Any>,
    ) -> Result<BuildFn, AssetError> {
        let src = src.downcast::<S>().expect(INVARIANT);
        let parsed = self
            .0
            .parse(&src, *raw.downcast::<S::Raw>().expect(INVARIANT))?;

        Ok(Box::new(move |handle, reg| {
            let h = handle.downcast_ref::<Handle<A>>().expect(INVARIANT);
            let built = self.0.build(&src, parsed)?;

            reg.as_any_mut()
                .downcast_mut::<Registry<A, <L as Loader<A, S>>::Built>>()
                .expect(INVARIANT)
                .update(*h, SlotState::Ready(built))
        }))
    }
}

pub trait Loader<A, S>: Send + Sync + 'static
where
    A: Asset,
    S: SourceFor<A>,
{
    type Built: 'static;
    type Parsed: Send + 'static;

    fn build(&self, src: &S, parsed: Self::Parsed) -> Result<Self::Built, AssetError>;
    fn parse(&self, src: &S, raw: S::Raw) -> Result<Self::Parsed, AssetError>;
}
