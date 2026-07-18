use std::{any::Any, marker::PhantomData};

use crate::assets::{
    Asset, AssetError, Handle, SourceFor,
    registry::{ErasedRegistry, Registry, SlotState},
};

pub(crate) trait ErasedLoader: Send + Sync {
    fn parse<'a>(
        &'a self,
        src: &(dyn Any + Send),
    ) -> Box<
        dyn FnOnce(&(dyn Any + Send), &mut dyn ErasedRegistry) -> Result<(), AssetError>
            + Send
            + 'a,
    >;
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
    S: SourceFor<A, Raw: Send> + Send,
    L: Loader<A, S> + Send + Sync,
{
    fn parse<'a>(
        &'a self,
        src: &(dyn Any + Send),
    ) -> Box<
        dyn FnOnce(&(dyn Any + Send), &mut dyn ErasedRegistry) -> Result<(), AssetError>
            + Send
            + 'a,
    > {
        let raw = src
            .downcast_ref::<S>()
            .expect("Mistyped source for LoaderCell")
            .fetch();
        Box::new(move |handle, reg| {
            let h = handle
                .downcast_ref::<Handle<A>>()
                .expect("Mistyped handle for LoaderCell");
            let built = self.0.build(raw);
            reg.as_any_mut()
                .downcast_mut::<Registry<A, <L as Loader<A, S>>::Built>>()
                .expect("Mistyped registry for LoaderCell")
                .update(*h, SlotState::Ready(built))
        })
    }
}

pub trait Loader<A, S>
where
    A: Asset,
    S: SourceFor<A, Raw: Send>,
{
    type Built: 'static;

    fn build(&self, raw: S::Raw) -> Self::Built;
}
