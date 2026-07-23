use std::{any::Any, marker::PhantomData, sync::Arc};

use crate::assets::{
    AssetError, BuiltAs, INVARIANT, SourceFor,
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

impl<B, S, L> ErasedLoader for LoaderCell<B, S, L>
where
    B: BuiltAs,
    S: SourceFor<B>,
    L: Loader<B, S>,
{
    fn parse(
        self: Arc<Self>,
        src: Arc<dyn Any + Send + Sync>,
        raw: Box<dyn Any>,
    ) -> Result<BuildFn, AssetError> {
        let src = src.downcast::<S>().expect(INVARIANT);
        let parsed = self.0.parse(
            &src,
            *raw.downcast::<S::Raw>().expect(INVARIANT),
            &mut Deps {},
        )?;

        Ok(Box::new(move |handle, reg| {
            let h = handle.downcast_ref::<Handle<B>>().expect(INVARIANT);
            let built = self.0.build(&src, parsed, &Fetch {})?;

            reg.as_any_mut()
                .downcast_mut::<Registry<B, B::Built>>()
                .expect(INVARIANT)
                .update(*h, SlotState::Ready(built))
        }))
    }
}

pub trait Loader<B, S>: Send + Sync + 'static
where
    B: BuiltAs,
    S: SourceFor<B>,
{
    type Parsed: Send + 'static;

    fn build(&self, src: &S, parsed: Self::Parsed, fetch: &Fetch) -> Result<B::Built, AssetError>;
    fn parse(&self, src: &S, raw: S::Raw, deps: &mut Deps) -> Result<Self::Parsed, AssetError>;
}

pub struct Deps;

pub struct Fetch;
