#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod winit;

use std::{error::Error, io};

use crate::renderer::Viewport;

pub(crate) trait HasViewport {
    fn get_viewport(&self) -> Viewport;
    fn request_redraw(&self);
    fn set_fullscreen_enabled(&self, enabled: bool);
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
