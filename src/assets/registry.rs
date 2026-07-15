use std::{
    any::{Any, type_name},
    error::Error,
    marker::PhantomData,
};

use crate::assets::AssetError;

pub(crate) trait ErasedSlotRegistry {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
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

struct Slot<T> {
    generation: u64,
    state: SlotState<T>,
}

impl<T> Slot<T> {
    fn new() -> Self {
        Self {
            generation: 0,
            state: SlotState::Pending,
        }
    }
}

pub enum SlotState<T> {
    Failed(Box<dyn Error + Send + Sync>),
    Pending,
    Ready(T),
}

pub(crate) struct SlotRegistry<T> {
    free: Vec<usize>,
    items: Vec<Slot<T>>,
}

impl<T> SlotRegistry<T> {
    pub(crate) fn new() -> Self {
        Self {
            free: Vec::new(),
            items: Vec::new(),
        }
    }

    pub(crate) fn get(&self, handle: &Handle<T>) -> Result<&SlotState<T>, AssetError> {
        if handle.id >= self.items.len() {
            return Err(AssetError::NotFound {
                t: type_name::<T>().to_string(),
                id: handle.id,
            });
        }

        let slot = &self.items[handle.id];
        if slot.generation != handle.generation {
            return Err(AssetError::NotFound {
                t: type_name::<T>().to_string(),
                id: handle.id,
            });
        }

        Ok(&slot.state)
    }

    pub(crate) fn insert(&mut self) -> Handle<T> {
        if let Some(id) = self.free.pop() {
            return Handle::new(id, self.items[id].generation);
        }

        self.items.push(Slot::new());
        Handle::new(self.items.len() - 1, 0)
    }

    pub(crate) fn release(&mut self, handle: Handle<T>) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;

        self.items[handle.id].generation += 1;
        self.items[handle.id].state = SlotState::Pending;
        self.free.push(handle.id);

        Ok(())
    }

    pub(crate) fn update(
        &mut self,
        handle: Handle<T>,
        state: SlotState<T>,
    ) -> Result<(), AssetError> {
        let _ = self.get(&handle)?;
        self.items[handle.id].state = state;
        Ok(())
    }
}

impl<T: 'static> ErasedSlotRegistry for SlotRegistry<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
