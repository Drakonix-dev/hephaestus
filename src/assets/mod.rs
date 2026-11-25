mod config;
mod errors;
mod loader;
mod pool;
mod reader;
mod registry;
mod store;

use std::panic::{self, AssertUnwindSafe};

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

// ----------------------------------------------------------------------------

fn catch_panic<T, F>(label: AssetLabel, cb: F) -> Result<T, AssetError>
where
    F: FnOnce() -> Result<T, AssetError>,
{
    panic::catch_unwind(AssertUnwindSafe(|| cb())).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic".into());
        Err(AssetError::Panicked {
            asset: label,
            message,
        })
    })
}
