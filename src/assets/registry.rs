use std::{
    any::{Any, type_name},
    error::Error,
    marker::PhantomData,
};

use crate::assets::{AssetError, AssetStatus};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    pub(crate) generation: u64,
    pub(crate) id: usize,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    pub(crate) fn new(id: usize, generation: u64) -> Self {
        Self {
            generation,
            id,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            generation: self.generation.clone(),
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}

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

impl<T> From<SlotState<T>> for AssetStatus {
    fn from(state: SlotState<T>) -> Self {
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
    fn insert_pending(&mut self) -> (usize, u64);
    fn release(&mut self, id: usize, generation: u64) -> Result<(), AssetError>;
    fn status(&self, id: usize, generation: u64) -> Result<AssetStatus, AssetError>;
}

pub(crate) struct Registry<Id, V> {
    free: Vec<usize>,
    items: Vec<Slot<V>>,
    _marker: PhantomData<fn() -> Id>,
}

impl<Id, V> Registry<Id, V> {
    pub(crate) fn new() -> Self {
        Self {
            free: Vec::new(),
            items: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub(crate) fn get(&self, handle: &Handle<Id>) -> Result<&SlotState<V>, AssetError> {
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

    pub(crate) fn insert(&mut self, state: SlotState<V>) -> Handle<Id> {
        if let Some(id) = self.free.pop() {
            return Handle::new(id, self.items[id].generation);
        }

        self.items.push(Slot::new(state));
        Handle::new(self.items.len() - 1, 0)
    }

    pub(crate) fn release(&mut self, handle: Handle<Id>) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;

        self.items[handle.id].generation += 1;
        self.items[handle.id].state = SlotState::Pending;
        self.free.push(handle.id);

        Ok(())
    }

    pub(crate) fn update(
        &mut self,
        handle: Handle<Id>,
        state: SlotState<V>,
    ) -> Result<(), AssetError> {
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
        self.get(&Handle {
            id,
            generation,
            _marker: PhantomData,
        })?
        .into()
    }
}

