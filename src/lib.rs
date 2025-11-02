mod api;
pub use api::*;

pub mod core;

#[proc_macro]
pub(crate) mod macros;

mod buffer;
mod platform;
mod renderer;

pub fn run<A: crate::app::Application + 'static>(app: A) {
    #[cfg(not(target_arch = "wasm32"))]
    crate::platform::winit::WinitPlatform::run(app);
}
