use crate::{
    assets::{Asset, registry::Handle},
    events::Event,
};

pub struct AssetFailed<A: Asset> {
    pub handle: Handle<A>,
    pub reason: String,
}

impl<A: Asset> Event for AssetFailed<A> {}

pub struct AssetLoaded<A: Asset> {
    pub handle: Handle<A>,
}

impl<A: Asset> Event for AssetLoaded<A> {}
