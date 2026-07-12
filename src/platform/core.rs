use std::{error, fmt, io, path};

use crate::EngineError;

pub(crate) trait HasWindowInfo {
    fn get_window_info(&self) -> WindowInfo;
    fn request_redraw(&self);
    fn set_fullscreen_enabled(&self, enabled: bool);
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowInfo {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug)]
pub enum PlatformError {
    AssetLoadFailed(path::PathBuf),
    AssetNotFound(String),
    BadAssetDefinition(String),
    BadRenderGraph(String),
    Frame(FrameError),
    IoError(io::Error),
    LoopError(String),
    RendererCreationFailed(String),
    WindowCreationFailed(String),
}

impl error::Error for PlatformError {}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<io::Error> for PlatformError {
    fn from(err: io::Error) -> Self {
        PlatformError::IoError(err)
    }
}

impl From<PlatformError> for EngineError {
    fn from(err: PlatformError) -> Self {
        match &err {
            PlatformError::AssetLoadFailed(path) => EngineError::Recoverable {
                message: format!("Failed to load asset: {}", path.display()),
                source: Some(Box::new(err)),
            },
            PlatformError::AssetNotFound(msg) => EngineError::Recoverable {
                message: format!("Failed to find asset: {msg}"),
                source: Some(Box::new(err)),
            },
            PlatformError::BadAssetDefinition(msg) => EngineError::Recoverable {
                message: msg.clone(),
                source: Some(Box::new(err)),
            },
            PlatformError::BadRenderGraph(msg) => EngineError::Fatal {
                message: msg.clone(),
                source: Some(Box::new(err)),
            },
            PlatformError::Frame(frame_err) => EngineError::Recoverable {
                message: frame_err.to_string(),
                source: Some(Box::new(err)),
            },
            PlatformError::IoError(io_err) => EngineError::Recoverable {
                message: io_err.to_string(),
                source: Some(Box::new(err)),
            },
            PlatformError::LoopError(msg) => EngineError::Fatal {
                message: msg.clone(),
                source: Some(Box::new(err)),
            },
            PlatformError::RendererCreationFailed(msg) => EngineError::Fatal {
                message: msg.clone(),
                source: Some(Box::new(err)),
            },
            PlatformError::WindowCreationFailed(msg) => EngineError::Fatal {
                message: msg.clone(),
                source: Some(Box::new(err)),
            },
        }
    }
}

#[derive(Debug)]
pub enum FrameError {
    OutOfMemory,
    OutdatedSurface,
    Other(String),
    SwapchainLost,
    Timeout,
}

impl error::Error for FrameError {}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<FrameError> for PlatformError {
    fn from(err: FrameError) -> Self {
        PlatformError::Frame(err)
    }
}
