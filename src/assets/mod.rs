mod registry;

use std::{
    any::{TypeId, type_name},
    collections::HashMap,
    error::Error,
};

use crate::assets::registry::{ErasedSlotRegistry, Handle, SlotRegistry, SlotState};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
enum AssetError {
    #[error("asset failed to load: {t}x{id}")]
    LoadingFailed {
        id: usize,
        #[source]
        source: Box<dyn Error + Send + Sync>,
        t: String,
    },

    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: String },

    #[error("asset unavailable: {t}x{id}")]
    Unavailable { id: usize, t: String },
}

pub trait Asset: 'static {}

pub struct Registry {
    slots: HashMap<TypeId, Box<dyn ErasedSlotRegistry>>,
}

impl Registry {
    pub(crate) fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }

    pub(crate) fn get<T: Asset>(&self, handle: Handle<T>) -> Result<&T, AssetError> {
        let state = self
            .slots
            .get(&TypeId::of::<T>())
            .and_then(|b| b.as_any().downcast_ref::<SlotRegistry<T>>())
            .map_or_else(
                || {
                    Err(AssetError::NotFound {
                        id: handle.id,
                        t: type_name::<T>().to_string(),
                    })
                },
                |r| r.get(&handle),
            )?;

        match state {
            SlotState::Failed(err) => Err(AssetError::LoadingFailed {
                id: handle.id,
                source: Box::new(err),
                t: type_name::<T>().to_string(),
            }),
            SlotState::Pending => Err(AssetError::Unavailable {
                id: handle.id,
                t: type_name::<T>().to_string(),
            }),
            SlotState::Ready(asset) => Ok(&asset),
        }
    }

    pub fn register<T: Asset>(&mut self) -> Handle<T> {
        self.slots
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(SlotRegistry::<T>::new()))
            .as_any_mut()
            .downcast_mut::<SlotRegistry<T>>()
            .expect("TypeId keyed the wrong registry")
            .insert()
    }
}
