use std::{
    any::Any,
    error::Error,
    marker::PhantomData,
    sync::mpsc::{self, Receiver, Sender},
};

use crate::assets::{Asset, AssetError, AssetStatus, Handle, INVARIANT, OwnedAsset};

pub(crate) struct Slot<T> {
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

pub(crate) enum SlotState<T> {
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

pub(crate) trait ErasedRegistry: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn failed(
        &mut self,
        handle: (usize, u64),
        err: Box<dyn Error + Send + Sync>,
    ) -> Result<(), AssetError>;
    fn insert_pending(&mut self) -> (usize, u64);
    fn ready(&mut self, handle: (usize, u64), v: Box<dyn Any>) -> Result<(), AssetError>;
    fn release(&mut self);
    fn status(&self, handle: (usize, u64)) -> Result<AssetStatus, AssetError>;
}

pub(crate) struct Registry<A: Asset, V> {
    free: Vec<usize>,
    items: Vec<Slot<V>>,
    _marker: PhantomData<fn() -> A>,
    rx: Receiver<usize>,
    tx: Sender<usize>,
}

impl<A: Asset, V> Registry<A, V> {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            free: Vec::new(),
            items: Vec::new(),
            rx,
            tx,
            _marker: PhantomData,
        }
    }

    fn get(&self, handle: &Handle<A>) -> Result<&SlotState<V>, AssetError> {
        if handle.id >= self.items.len() {
            return Err(handle.not_found());
        }

        let slot = &self.items[handle.id];
        if slot.generation != handle.generation {
            return Err(handle.not_found());
        }

        Ok(&slot.state)
    }

    fn insert(&mut self, state: SlotState<V>) -> OwnedAsset<A> {
        let handle = if let Some(id) = self.free.pop() {
            Handle::new(id, self.items[id].generation)
        } else {
            self.items.push(Slot::new(state));
            Handle::new(self.items.len() - 1, 0)
        };

        OwnedAsset::new(self.tx.clone(), handle)
    }

    fn release(&mut self) {
        while let Ok(id) = self.rx.try_recv() {
            self.items[id].generation += 1;
            self.items[id].state = SlotState::Pending;
            self.free.push(id);
        }
    }

    pub(crate) fn update(
        &mut self,
        handle: Handle<A>,
        state: SlotState<V>,
    ) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;
        self.items[handle.id].state = state;
        Ok(())
    }
}

impl<A: Asset, V: 'static> ErasedRegistry for Registry<A, V> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn failed(
        &mut self,
        handle: (usize, u64),
        err: Box<dyn Error + Send + Sync>,
    ) -> Result<(), AssetError> {
        self.update(Handle::new(handle.0, handle.1), SlotState::Failed(err))
    }

    fn insert_pending(&mut self) -> (usize, u64) {
        let asset = self.insert(SlotState::Pending);
        let handle = asset.handle();
        (handle.id, handle.generation)
    }

    fn ready(&mut self, handle: (usize, u64), v: Box<dyn Any>) -> Result<(), AssetError> {
        let state = v.downcast::<V>().expect(INVARIANT);
        self.update(Handle::new(handle.0, handle.1), SlotState::Ready(*state))
    }

    fn release(&mut self) {
        self.release()
    }

    fn status(&self, handle: (usize, u64)) -> Result<AssetStatus, AssetError> {
        self.get(&Handle::new(handle.0, handle.1))
            .map(|s| AssetStatus::from(s))
    }
}
