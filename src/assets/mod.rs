pub mod rework;

mod registry;

pub trait Asset: 'static {}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
enum AssetError {
    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: String },
}
