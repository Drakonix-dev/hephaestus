mod config;
mod errors;
mod loader;
mod pool;
mod reader;
mod registry;
mod store;

pub use config::*;
pub use errors::*;
pub use loader::*;
pub use pool::*;
pub use reader::*;
pub use registry::*;
pub use store::*;

const INVARIANT: &str = "asset type-erasure invariant violated";

pub trait GameAsset: 'static {
    type Built: 'static;
}

impl<T: GameAsset> private::BuiltAsset for T {
    type Built = <T as GameAsset>::Built;
}

impl<T: GameAsset> Asset for T {}
