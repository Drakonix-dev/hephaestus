#[derive(Debug, Clone, Copy)]
pub enum Event {
    Resized(u32, u32),
    Quit,
}
