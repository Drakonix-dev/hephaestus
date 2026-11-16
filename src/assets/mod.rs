use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    error::Error,
    marker::PhantomData,
};

pub(crate) mod private {
    pub trait BuiltAsset: 'static {
        type Built: 'static;
    }
}

const INVARIANT: &str = "asset type-erasure invariant violated";

pub trait Asset: private::BuiltAsset {}

pub trait GameAsset: 'static {
    type Built: 'static;
}

impl<T: GameAsset> private::BuiltAsset for T {
    type Built = <T as GameAsset>::Built;
}

impl<T: GameAsset> Asset for T {}

pub enum AssetStatus {
    Failed(String),
    Pending,
    Ready,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: &'static str },

    #[error("asset {t} cannot be handled for source {s}")]
    UnhandledAsset { t: &'static str, s: &'static str },
}

impl AssetError {
    fn not_found<A: Asset>(handle: &Handle<A>) -> Self {
        Self::NotFound {
            id: handle.id,
            t: type_name::<A>(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T: Asset> {
    pub(crate) generation: u64,
    pub(crate) id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Asset> Handle<T> {
    pub(crate) fn new(id: usize, generation: u64) -> Self {
        Self {
            generation,
            id,
            _marker: PhantomData,
        }
    }
}

impl<T: Asset> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation.clone(),
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

pub struct SourceId(pub &'static str);

pub trait Source: 'static {
    type Output;

    fn id(&self) -> SourceId;
    fn read(&self) -> Result<Self::Output, AssetError>;
}

type BuildFn = Box<dyn FnOnce(&dyn Any, &mut dyn ErasedStore) -> Result<(), AssetError>>;

trait ErasedLoader {
    fn parse(self: Self, src: Box<dyn Any>) -> Result<BuildFn, AssetError>;
}

pub trait Loader<A: Asset>: 'static {
    type Parsed: 'static;
    type Raw;

    fn build(&self, parsed: Self::Parsed) -> Result<A::Built, AssetError>;
    fn parse(&self, raw: Self::Raw) -> Result<Self::Parsed, AssetError>;
}

struct LoaderCell<A, S, L>(L, PhantomData<fn() -> (A, S)>);

impl<A, S, L> LoaderCell<A, S, L> {
    fn new(l: L) -> Self {
        Self(l, PhantomData)
    }
}

impl<A, S, L> ErasedLoader for LoaderCell<A, S, L>
where
    A: Asset,
    S: Source,
    L: Loader<A, Raw = S::Output>,
{
    fn parse(self: Self, src: Box<dyn Any>) -> Result<BuildFn, AssetError> {
        let raw = src.downcast::<S>().expect(INVARIANT).read()?;
        let parsed = self.0.parse(raw)?;

        Ok(Box::new(move |handle, store| {
            let h = handle.downcast_ref::<Handle<A>>().expect(INVARIANT);
            let built = self.0.build(parsed)?;

            store
                .as_any_mut()
                .downcast_mut::<Store<A>>()
                .expect(INVARIANT)
                .update(*h, SlotState::Ready(built))
        }))
    }
}

struct Slot<T> {
    generation: u64,
    state: SlotState<T>,
}

impl<T> Slot<T> {
    fn new(state: SlotState<T>) -> Self {
        Self {
            generation: 0,
            state,
        }
    }
}

enum SlotState<T> {
    Failed(Box<dyn Error + Send + Sync>),
    Pending,
    Ready(T),
}

impl<T> From<&SlotState<T>> for AssetStatus {
    fn from(state: &SlotState<T>) -> Self {
        match state {
            SlotState::Failed(err) => AssetStatus::Failed(err.to_string()),
            SlotState::Pending => AssetStatus::Pending,
            SlotState::Ready(_) => AssetStatus::Ready,
        }
    }
}

struct Registry {
    loaders: HashMap<(TypeId, TypeId), Box<dyn ErasedLoader>>,
    stores: HashMap<TypeId, Box<dyn ErasedStore>>,
}

impl Registry {
    fn new() -> Self {
        Self {
            loaders: HashMap::new(),
            stores: HashMap::new(),
        }
    }

    fn load<A: Asset, S: Source>(&mut self, src: S) -> Result<Handle<A>, AssetError> {
        let asset_key = TypeId::of::<A>();
        let key = (asset_key, TypeId::of::<S::Output>());
        let loader = self.loaders.get(&key).ok_or(AssetError::UnhandledAsset {
            t: type_name::<A>(),
            s: type_name::<S>(),
        })?;

        let store = self
            .stores
            .entry(asset_key)
            .or_insert_with(|| Box::new(Store::<A>::new()));

        let (id, generation) = store.insert_pending();
        let handle = Handle::new(id, generation);

        let raw = src.read()?;
        let build = loader.parse(Box::new(raw))?;

        build(&handle, store.as_mut())?;
        Ok(handle)
    }
}

trait ErasedStore: Any {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn insert_pending(&mut self) -> (usize, u64);
}

struct Store<A: Asset> {
    free: Vec<usize>,
    items: Vec<Slot<A::Built>>,
}

impl<A: Asset> Store<A> {
    fn new() -> Self {
        Self {
            free: Vec::new(),
            items: Vec::new(),
        }
    }

    fn get(&self, handle: &Handle<A>) -> Result<&SlotState<A::Built>, AssetError> {
        if handle.id >= self.items.len() {
            return Err(AssetError::not_found(handle));
        }

        let slot = &self.items[handle.id];
        if slot.generation != handle.generation {
            return Err(AssetError::not_found(handle));
        }

        Ok(&slot.state)
    }

    fn insert(&mut self, state: SlotState<A::Built>) -> Handle<A> {
        if let Some(id) = self.free.pop() {
            Handle::new(id, self.items[id].generation)
        } else {
            self.items.push(Slot::new(state));
            Handle::new(self.items.len() - 1, 0)
        }
    }

    fn update(&mut self, handle: Handle<A>, state: SlotState<A::Built>) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;
        self.items[handle.id].state = state;
        Ok(())
    }
}

impl<A: Asset> ErasedStore for Store<A> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn insert_pending(&mut self) -> (usize, u64) {
        let handle = self.insert(SlotState::Pending);
        (handle.id, handle.generation)
    }
}
