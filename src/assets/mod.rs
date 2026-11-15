use std::{any::type_name, error::Error, marker::PhantomData};

pub(crate) mod private {
    pub trait BuiltAsset: 'static {
        type Built: 'static;
    }
}

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

pub trait Loader<A: Asset>: 'static {
    type Parsed: 'static;
    type Raw;

    fn build(&self, parsed: Self::Parsed) -> Result<A::Built, AssetError>;
    fn parse(&self, raw: Self::Raw) -> Result<Self::Parsed, AssetError>;
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

struct Registry<A: Asset> {
    free: Vec<usize>,
    items: Vec<Slot<A::Built>>,
}

impl<A: Asset> Registry<A> {
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
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     struct MyAsset;
//     impl GameAsset for MyAsset {
//         type Built = MyAsset;
//     }
//
//     #[test]
//     fn loads_asset() {}
// }
