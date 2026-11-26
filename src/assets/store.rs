use core::fmt;
use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::{Arc, Weak},
};

use crossbeam_channel::Sender;

use crate::assets::{
    AssetError, INVARIANT,
    ticket::{Allocator, Ticket},
};

pub(crate) mod private {
    pub trait BuiltAsset: 'static {
        type Built: 'static;
    }
}

pub trait Asset: private::BuiltAsset {}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AssetLabel(Arc<str>);

impl AssetLabel {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AssetLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for AssetLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl From<&str> for AssetLabel {
    fn from(s: &str) -> Self {
        Self(Arc::from(s))
    }
}

impl From<String> for AssetLabel {
    fn from(s: String) -> Self {
        Self(Arc::from(s))
    }
}

#[derive(Default)]
pub(super) struct Stores(HashMap<TypeId, Box<dyn ErasedStore>>);

impl Stores {
    pub(super) fn get_built<A: Asset>(
        &self,
        key: &Key<A>,
    ) -> Option<&AssetState<A::Built>> {
        self.0
            .get(&TypeId::of::<A>())
            .expect("handle minted by different registry")
            .as_any()
            .downcast_ref::<Store<A>>()
            .expect(INVARIANT)
            .lookup(&key)
    }

    pub(super) fn get_erased(
        &mut self,
        id: &TypeId,
    ) -> Option<&mut Box<dyn ErasedStore>> {
        self.0.get_mut(&id)
    }

    pub(super) fn get_mut<A: Asset>(&mut self) -> &mut Store<A> {
        self.0
            .get_mut(&TypeId::of::<A>())
            .expect(INVARIANT)
            .as_any_mut()
            .downcast_mut::<Store<A>>()
            .expect(INVARIANT)
    }

    pub(super) fn init<A: Asset>(&mut self, allocator: Arc<Allocator>) {
        let _ = self
            .0
            .entry(TypeId::of::<A>())
            .or_insert_with(|| Box::new(Store::<A>::new(allocator)));
    }
}

pub(super) trait ErasedStore: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn free(&mut self, freed: Freed);
}

#[derive(Default)]
pub(super) struct Store<A: Asset> {
    allocator: Arc<Allocator>,
    items: Vec<Slot<A>>,
}

impl<A: Asset> Store<A> {
    fn new(allocator: Arc<Allocator>) -> Self {
        Self {
            allocator,
            items: Vec::new(),
        }
    }

    fn free(&mut self, freed: Freed) {
        if let Some(slot) = self.items.get_mut(freed.id) {
            if slot.generation != freed.generation {
                return;
            }

            slot.generation += 1;
            slot.guard = Weak::new();
            slot.state = None;

            self.allocator.release(Ticket {
                id: freed.id,
                generation: freed.generation,
            })
        }
    }

    pub(super) fn insert(&mut self, tx: Sender<Freed>) -> Handle<A> {
        let t = self.allocator.alloc();
        if t.id >= self.items.len() {
            self.items.resize_with(t.id + 1, || Slot::new(Weak::new()));
        }

        let key = Key::new(t.id, t.generation);
        let guard = Arc::new(Guard::new(TypeId::of::<A>(), key, tx));

        self.items[t.id].generation = t.generation;
        self.items[t.id].guard = Arc::downgrade(&guard);
        self.items[t.id].state = Some(AssetState::Pending);

        Handle(guard)
    }

    pub(super) fn lookup(&self, key: &Key<A>) -> Option<&AssetState<A::Built>> {
        if let Some(slot) = self.items.get(key.id) {
            if slot.generation == key.generation {
                return slot.state.as_ref();
            } else if slot.generation > key.generation {
                return None;
            }
        }

        Some(&AssetState::Pending)
    }

    pub(super) fn update(&mut self, key: &Key<A>, state: AssetState<A::Built>) {
        if let Some(slot) = self.items.get_mut(key.id) {
            if slot.generation == key.generation {
                slot.state = Some(state);
            }
        }
    }
}

impl<A: Asset> ErasedStore for Store<A> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn free(&mut self, freed: Freed) {
        self.free(freed)
    }
}

struct Slot<A: Asset> {
    generation: u64,
    guard: Weak<Guard<A>>,
    state: Option<AssetState<A::Built>>,
}

impl<A: Asset> Slot<A> {
    fn new(guard: Weak<Guard<A>>) -> Self {
        Self {
            generation: 0,
            guard,
            state: None,
        }
    }
}

#[non_exhaustive]
pub enum AssetState<T> {
    Failed(Arc<AssetError>),
    Pending,
    Ready(T),
}

impl<T> AssetState<T> {
    pub fn status(&self) -> AssetStatus {
        match self {
            AssetState::Failed(err) => AssetStatus::Failed(err.clone()),
            AssetState::Pending => AssetStatus::Pending,
            AssetState::Ready(_) => AssetStatus::Ready,
        }
    }
}

#[non_exhaustive]
pub enum AssetStatus {
    Failed(Arc<AssetError>),
    Pending,
    Ready,
}

pub struct Handle<A: Asset>(pub(super) Arc<Guard<A>>);

impl<A: Asset> Handle<A> {
    pub fn downgrade(&self) -> WeakHandle<A> {
        WeakHandle(self.0.key)
    }

    pub(super) fn key(&self) -> Key<A> {
        self.0.key
    }
}

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<A: Asset> Eq for Handle<A> {}

impl<A: Asset> Hash for Handle<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.key.hash(state);
    }
}

impl<A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.key.eq(&other.0.key)
    }
}

pub struct WeakHandle<A: Asset>(pub(super) Key<A>);

impl<A: Asset> WeakHandle<A> {
    pub(super) fn key(&self) -> Key<A> {
        self.0
    }
}

impl<A: Asset> Clone for WeakHandle<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: Asset> Copy for WeakHandle<A> {}

impl<A: Asset> Eq for WeakHandle<A> {}

impl<A: Asset> Hash for WeakHandle<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<A: Asset> PartialEq for WeakHandle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<A: Asset> From<&Handle<A>> for WeakHandle<A> {
    fn from(handle: &Handle<A>) -> Self {
        Self(handle.key())
    }
}

pub(super) struct Key<T> {
    pub(super) generation: u64,
    pub(super) id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Key<T> {
    pub(super) fn new(id: usize, generation: u64) -> Self {
        Self {
            id,
            generation,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for Key<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Key<T> {}

impl<T> Eq for Key<T> {}

impl<T> Hash for Key<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.generation.hash(state);
    }
}

impl<T> PartialEq for Key<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.generation == other.generation
    }
}

pub(super) struct Guard<T> {
    id: TypeId,
    pub(super) key: Key<T>,
    tx: Sender<Freed>,
}

impl<T> Guard<T> {
    pub(super) fn new(id: TypeId, key: Key<T>, tx: Sender<Freed>) -> Self {
        Self { id, key, tx }
    }
}

impl<T> Drop for Guard<T> {
    fn drop(&mut self) {
        let _ = self.tx.send(Freed {
            type_id: self.id,
            type_name: type_name::<T>(),
            id: self.key.id,
            generation: self.key.generation,
        });
    }
}

pub(super) struct Freed {
    pub(super) type_id: TypeId,
    pub(super) type_name: &'static str,
    pub(super) id: usize,
    pub(super) generation: u64,
}
