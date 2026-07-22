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
    S: SourceFor<A, Raw: Send>,
    L: Loader<A, S, Parsed: Send> + Send + Sync + 'static,
{
    fn parse(
        self: Arc<Self>,
        src: Arc<dyn Any + Send + Sync>,
        raw: Box<dyn Any>,
    ) -> Result<BuildFn, AssetError> {
        let parsed = self.0.parse(
            &*src.downcast::<S>().expect(INVARIANT),
            *raw.downcast::<S::Raw>().expect(INVARIANT),
        )?;

        Ok(Box::new(move |handle, reg| {
            let h = handle.downcast_ref::<Handle<A>>().expect(INVARIANT);
            let built = self.0.build(parsed)?;

            reg.as_any_mut()
                .downcast_mut::<Registry<A, <L as Loader<A, S>>::Built>>()
                .expect(INVARIANT)
                .update(*h, SlotState::Ready(built))
        }))
    }
}

pub trait Loader<A, S>
where
    A: Asset,
    S: SourceFor<A, Raw: Send>,
{
    type Built: 'static;
    type Parsed: 'static;

    fn parse(&self, src: &S, raw: S::Raw) -> Result<Self::Parsed, AssetError>;
    fn build(&self, parsed: Self::Parsed) -> Result<Self::Built, AssetError>;
}
