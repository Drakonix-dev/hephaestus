use std::{any::Any, marker::PhantomData, sync::Arc};

use crate::assets::{
    AssetError, BuiltAs, Dependency, Handle, INVARIANT, Priority, SourceFor,
    registry::{ErasedRegistry, Registry, SlotState},
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
            &mut Deps::new(),
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

trait ErasedDependency: Any {}
trait ErasedHandle: Any {}

struct DependencyCell<B: BuiltAs, S: SourceFor<B>> {
    priority: Priority,
    src: S,
    _marker: PhantomData<fn() -> B>,
}

impl<B: BuiltAs, S: SourceFor<B>> DependencyCell<B, S> {
    fn new(src: S, priority: Priority) -> Self {
        Self {
            priority,
            src,
            _marker: PhantomData,
        }
    }
}

impl<B: BuiltAs, S: SourceFor<B>> ErasedDependency for DependencyCell<B, S> {}

struct HandleCell<B: BuiltAs> {
    handle: Handle<B>,
    priority: Priority,
}

impl<B: BuiltAs> HandleCell<B> {
    fn new(handle: Handle<B>, priority: Priority) -> Self {
        Self { handle, priority }
    }
}

impl<B: BuiltAs> ErasedHandle for HandleCell<B> {}

pub struct Deps {
    deps: Vec<Box<dyn ErasedDependency>>,
    handles: Vec<Box<dyn ErasedHandle>>,
}

impl Deps {
    fn new() -> Self {
        Self {
            deps: Vec::new(),
            handles: Vec::new(),
        }
    }

    pub fn require<B: BuiltAs, S: SourceFor<B>>(
        &mut self,
        src: S,
        priority: Priority,
    ) -> Dependency<B> {
        self.deps.push(Box::new(DependencyCell::new(src, priority)));
        Dependency::new(self.deps.len())
    }

    pub fn require_handle<B: BuiltAs>(&mut self, handle: Handle<B>, priority: Priority) {
        self.handles
            .push(Box::new(HandleCell::new(handle, priority)));
    }
}

pub struct Fetch;
