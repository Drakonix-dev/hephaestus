use std::{any::Any, marker::PhantomData};

use crate::assets::{
    Asset, AssetError, Handle, SourceFor,
    registry::{ErasedRegistry, Registry, SlotState},
};

pub(crate) trait ErasedLoader {
    fn parse<'a>(
        &'a self,
        raw: Box<dyn Any>,
    ) -> Box<
        dyn for<'b> FnOnce(
                &'b (dyn Any + 'b),
                &'b mut (dyn ErasedRegistry + 'b),
            ) -> Result<(), AssetError>
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
    S: SourceFor<A>,
    L: Loader<A, S>,
{
    fn parse<'a>(
        &'a self,
        raw: Box<dyn Any>,
    ) -> Box<
        dyn for<'b> FnOnce(
                &'b (dyn Any + 'b),
                &'b mut (dyn ErasedRegistry + 'b),
            ) -> Result<(), AssetError>
            + 'a,
    > {
        let parsed = self.0.parse(
            *raw.downcast::<S::Raw>()
                .expect("Mistyped raw for LoaderCell"),
        );

        Box::new(move |handle, reg| {
            let h = handle
                .downcast_ref::<Handle<A>>()
                .expect("Mistyped handle for LoaderCell");
            let built = self.0.build(parsed);

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
    S: SourceFor<A>,
{
    type Built: 'static;
    type Parsed: 'static;

    fn parse(&self, raw: S::Raw) -> Self::Parsed;
    fn build(&self, parsed: Self::Parsed) -> Self::Built;
}
