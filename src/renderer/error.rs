use std::{error::Error, path::PathBuf};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    #[error("asset not found: {name}")]
    AssetNotFound { name: String },

    #[error("failed to load asset '{}'", path.display())]
    AssetLoad {
        path: PathBuf,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error("invalid asset definition: {detail}")]
    BadAsset { detail: String },

    #[error("invalid render graph: {detail}")]
    BadRenderGraph { detail: String },

    #[error("renderer creation failed")]
    Creation {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error(transparent)]
    Frame(#[from] FrameError),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FrameError {
    #[error("{0}")]
    Other(String),

    #[error("surface out of memory")]
    OutOfMemory,

    #[error("surface outdated")]
    OutdatedSurface,

    #[error("surface lost")]
    SwapchainLost,

    #[error("frame timed out")]
    Timeout,
}
