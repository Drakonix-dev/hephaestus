#[derive(Debug, Clone, Copy)]
pub enum Event {
    CloseRequested,
    Resized(u32, u32),
    Redraw,
}
