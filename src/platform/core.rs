use std::{error::Error, io};

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

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PlatformError {
    #[error("event loop failed")]
    EventLoop {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("window creation failed")]
    WindowCreation {
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },
}
