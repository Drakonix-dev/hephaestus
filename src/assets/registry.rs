use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    error::Error,
    marker::PhantomData,
};

use crate::{
    assets::{Asset, AssetError},
    macros::{erased_downcast, erased_entry, erased_get},
};

pub struct Registry {
    slots: HashMap<TypeId, Box<dyn ErasedSlotRegistry>>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub fn add<T: Asset>(&mut self, asset: T) -> Handle<T> {
        erased_entry!(self.slots, SlotRegistry, T).insert(SlotState::Ready(asset))
    }

    pub(crate) fn get<T: Asset>(&self, handle: Handle<T>) -> Result<&SlotState<T>, AssetError> {
        erased_get!(self.slots, SlotRegistry, T).map_or_else(
            || {
                Err(AssetError::NotFound {
                    id: handle.id,
                    t: type_name::<T>().to_string(),
                })
            },
            |r| r.get(&handle),
        )
    }

    pub fn load<T: Asset>(&mut self) -> Handle<T> {
        erased_entry!(self.slots, SlotRegistry, T).insert(SlotState::Pending)
    }
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
    fn new(state: SlotState<T>) -> Self {
        Self {
            generation: 0,
            state,
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

erased_downcast!(SlotRegistry);

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

    pub(crate) fn insert(&mut self, state: SlotState<T>) -> Handle<T> {
        if let Some(id) = self.free.pop() {
            return Handle::new(id, self.items[id].generation);
        }

        self.items.push(Slot::new(state));
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
