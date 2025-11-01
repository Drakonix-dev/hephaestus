use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

pub(crate) struct WindowHandle {
    pub(crate) display_handle: RawDisplayHandle,
    pub(crate) window_handle: RawWindowHandle,
}

impl WindowHandle {
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Self {
        Self {
            display_handle,
            window_handle,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowInfo {
    pub(crate) width: u32,
    pub(crate) height: u32,
}
