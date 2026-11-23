use core::fmt;
use std::{
    any::Any,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
};

use crate::assets::AssetError;

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

pub enum AssetStatus {
    Failed(Arc<AssetError>),
    Pending,
    Ready,
}

pub(super) trait ErasedStore: Any {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn insert_pending(&mut self) -> (usize, u64);
}

pub(super) struct Store<A: Asset> {
    free: Vec<usize>,
    items: Vec<Slot<A::Built>>,
}

impl<A: Asset> Store<A> {
    pub(super) fn new() -> Self {
        Self {
            free: Vec::new(),
            items: Vec::new(),
        }
    }

    fn get(&self, handle: &Handle<A>) -> Option<&SlotState<A::Built>> {
        if handle.0.id >= self.items.len() {
            return None;
        }

        let slot = &self.items[handle.0.id];
        if slot.generation != handle.0.generation {
            return None;
        }

        Some(&slot.state)
    }

    fn insert(&mut self, state: SlotState<A::Built>) -> Handle<A> {
        if let Some(id) = self.free.pop() {
            let slot = &mut self.items[id];
            slot.state = state;
            Handle(Token::new(id, slot.generation))
        } else {
            self.items.push(Slot::new(state));
            Handle(Token::new(self.items.len() - 1, 0))
        }
    }

    pub(super) fn update(
        &mut self,
        handle: Handle<A>,
        state: SlotState<A::Built>,
    ) {
        if self.get(&handle).is_some() {
            self.items[handle.0.id].state = state;
        }
    }
}

impl<A: Asset> ErasedStore for Store<A> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn insert_pending(&mut self) -> (usize, u64) {
        let handle = self.insert(SlotState::Pending);
        (handle.0.id, handle.0.generation)
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

pub(super) enum SlotState<T> {
    Failed(Arc<AssetError>),
    Pending,
    Ready(T),
}

impl<T> From<&SlotState<T>> for AssetStatus {
    fn from(state: &SlotState<T>) -> Self {
        match state {
            SlotState::Failed(err) => AssetStatus::Failed(err.clone()),
            SlotState::Pending => AssetStatus::Pending,
            SlotState::Ready(_) => AssetStatus::Ready,
        }
    }
}

pub struct Handle<T: Asset>(pub(super) Token<T>);
pub struct OwnedHandle<T: Asset>(pub(super) Arc<Token<T>>);

struct Token<T: Asset> {
    generation: u64,
    id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T: Asset> Token<T> {
    fn new(id: usize, generation: u64) -> Self {
        Self {
            id,
            generation,
            _marker: PhantomData,
        }
    }
}

impl<T: Asset> Eq for Token<T> {}

impl<T: Asset> Hash for Token<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.generation.hash(state);
        self.id.hash(state);
    }
}

impl<T: Asset> PartialEq for Token<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.generation == other.generation
    }
}
