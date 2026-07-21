use std::error::Error;

use crate::assets::AssetError;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    #[error(transparent)]
    Asset(#[from] AssetError),

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
