mod catalog;
mod registry;

pub trait Asset: 'static {}

pub struct AssetId(pub(crate) u64);

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
enum AssetError {
    #[error("asset not found: {t}x{id}")]
    NotFound { id: usize, t: String },
}
