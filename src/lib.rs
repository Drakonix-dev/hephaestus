mod api;
pub use api::*;

pub mod core;

pub(crate) mod macros;

mod buffer;
mod platform;
mod renderer;

#[cfg(not(target_arch = "wasm32"))]
mod platform_winit;

pub fn run<A: crate::app::Application + 'static>(app: A) {
    #[cfg(not(target_arch = "wasm32"))]
    platform_winit::WinitPlatform::run(app);
}
