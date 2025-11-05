mod api;
pub use api::*;

pub mod events;
pub mod math;

pub(crate) mod macros;

mod platform;
mod renderer;

pub fn run<A: crate::Application + 'static>(app: A) {
    #[cfg(not(target_arch = "wasm32"))]
    crate::platform::winit::WinitPlatform::run(app);
}
