use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    error::Error,
    marker::PhantomData,
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AssetError {
    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: String },

    #[error("asset cannot be handled: {t}")]
    UnhandledAsset { t: String },

    #[error("asset type unknown: {t}")]
    UnknownAsset { t: String },
}

pub enum AssetStatus {
    Failed(String),
    Pending,
    Ready,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub(crate) generation: u64,
    pub(crate) id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    fn new(id: usize, generation: u64) -> Self {
        Self {
            generation,
            id,
            _marker: PhantomData,
        }
    }
}

pub struct Manager {
    loaders: HashMap<TypeId, Box<dyn ErasedLoader>>,
    registry: HashMap<TypeId, Box<dyn ErasedRegistry>>,
}

impl Manager {
    fn new() -> Self {
        Self {
            loaders: HashMap::new(),
            registry: HashMap::new(),
        }
    }

    pub fn load<A: Asset, S: SourceFor<A> + Send>(
        &mut self,
        src: S,
    ) -> Result<Handle<A>, AssetError> {
        let (id, generation) = self
            .registry
            .get_mut(&TypeId::of::<A>())
            .ok_or(AssetError::UnknownAsset {
                t: type_name::<A>().to_string(),
            })?
            .insert_pending();

        let _ = self
            .loaders
            .get(&TypeId::of::<A>())
            .ok_or(AssetError::UnhandledAsset {
                t: type_name::<A>().to_string(),
            })?
            .parse(&src);

        Ok(Handle::new(id, generation))
    }

    fn register<A, S, L>(&mut self, l: L)
    where
        A: Asset,
        S: SourceFor<A> + Send,
        L: Loader<A, S> + Send + Sync + 'static,
    {
        self.registry
            .entry(TypeId::of::<A>())
            .or_insert_with(|| Box::new(Registry::<A, L::Built>::new()));
        self.loaders
            .entry(TypeId::of::<A>())
            .or_insert_with(|| Box::new(LoaderCell::<A, S, L>::new(l)));
    }

    pub fn status<A: Asset>(&self, handle: Handle<A>) -> Result<AssetStatus, AssetError> {
        self.registry
            .get(&TypeId::of::<A>())
            .ok_or(AssetError::NotFound {
                id: handle.id,
                t: type_name::<A>().to_string(),
            })?
            .status(handle.id, handle.generation)
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

trait ErasedRegistry: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn insert_pending(&mut self) -> (usize, u64);
    fn release(&mut self, id: usize, generation: u64) -> Result<(), AssetError>;
    fn status(&self, id: usize, generation: u64) -> Result<AssetStatus, AssetError>;
}

struct Registry<Id, V> {
    free: Vec<usize>,
    items: Vec<Slot<V>>,
    _marker: PhantomData<fn() -> Id>,
}

impl<Id, V> Registry<Id, V> {
    fn new() -> Self {
        Self {
            free: Vec::new(),
            items: Vec::new(),
            _marker: PhantomData,
        }
    }

    fn get(&self, handle: &Handle<Id>) -> Result<&SlotState<V>, AssetError> {
        if handle.id >= self.items.len() {
            return Err(AssetError::NotFound {
                t: type_name::<Id>().to_string(),
                id: handle.id,
            });
        }

        let slot = &self.items[handle.id];
        if slot.generation != handle.generation {
            return Err(AssetError::NotFound {
                t: type_name::<Id>().to_string(),
                id: handle.id,
            });
        }

        Ok(&slot.state)
    }

    fn insert(&mut self, state: SlotState<V>) -> Handle<Id> {
        if let Some(id) = self.free.pop() {
            return Handle::new(id, self.items[id].generation);
        }

        self.items.push(Slot::new(state));
        Handle::new(self.items.len() - 1, 0)
    }

    fn release(&mut self, handle: Handle<Id>) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;

        self.items[handle.id].generation += 1;
        self.items[handle.id].state = SlotState::Pending;
        self.free.push(handle.id);

        Ok(())
    }

    fn update(&mut self, handle: Handle<Id>, state: SlotState<V>) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;
        self.items[handle.id].state = state;
        Ok(())
    }
}

impl<Id: 'static, V: 'static> ErasedRegistry for Registry<Id, V> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn insert_pending(&mut self) -> (usize, u64) {
        let handle = self.insert(SlotState::Pending);
        (handle.id, handle.generation)
    }

    fn release(&mut self, id: usize, generation: u64) -> Result<(), AssetError> {
        self.release(Handle {
            id,
            generation,
            _marker: PhantomData,
        })
    }

    fn status(&self, id: usize, generation: u64) -> Result<AssetStatus, AssetError> {
        let state = self.get(&Handle {
            id,
            generation,
            _marker: PhantomData,
        })?;

        match state {
            SlotState::Failed(err) => Ok(AssetStatus::Failed(err.to_string())),
            SlotState::Pending => Ok(AssetStatus::Pending),
            SlotState::Ready(_) => Ok(AssetStatus::Ready),
        }
    }
}

pub trait Asset: 'static {}

pub trait SourceFor<A: Asset>: 'static {
    type Raw: Send;
    fn fetch(&self) -> Self::Raw;
}

trait ErasedLoader: Send + Sync {
    fn parse<'a>(&'a self, src: &(dyn Any + Send)) -> Box<dyn FnOnce(&mut Manager) + Send + 'a>;
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
    S: SourceFor<A, Raw: Send> + Send,
    L: Loader<A, S> + Send + Sync,
{
    fn parse<'a>(&'a self, src: &(dyn Any + Send)) -> Box<dyn FnOnce(&mut Manager) + Send + 'a> {
        let raw = src
            .downcast_ref::<S>()
            .expect("Mistyped source for LoaderCell")
            .fetch();
        Box::new(move |mgr| {
            let built = self.0.build(raw);
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

// ----------------------------------------------------------------------------

pub struct Material;
impl Asset for Material {}

pub struct MaterialDesc;
impl SourceFor<Material> for MaterialDesc {
    type Raw = Self;

    fn fetch(&self) -> Self {
        Self {}
    }
}

pub struct MaterialImpl;

pub struct MaterialLoader;
impl Loader<Material, MaterialDesc> for MaterialLoader {
    type Built = MaterialImpl;

    fn build(&self, _: <MaterialDesc as SourceFor<Material>>::Raw) -> MaterialImpl {
        MaterialImpl {}
    }
}
