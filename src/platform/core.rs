use std::{error, fmt};

use crate::EngineError;

pub(crate) trait HasWindowInfo {
    fn get_window_info(&self) -> WindowInfo;
    fn request_redraw(&self);
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowInfo {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug)]
pub enum PlatformError {
    WindowCreationFailed(String),
}

impl error::Error for PlatformError {}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<PlatformError> for EngineError {
    fn from(err: PlatformError) -> Self {
        match &err {
            PlatformError::WindowCreationFailed(msg) => EngineError::Fatal {
                message: msg.clone(),
                source: Some(Box::new(err)),
            },
        }
    }
}
