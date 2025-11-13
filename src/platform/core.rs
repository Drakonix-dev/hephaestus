#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowInfo {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

pub(crate) trait HasWindowInfo {
    fn get_window_info(&self) -> WindowInfo;
    fn request_redraw(&self);
}
