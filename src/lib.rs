mod api;
mod platform;
mod renderer;

pub(crate) mod macros;

pub mod events;
pub mod math;

pub use api::*;

use crate::renderer::RenderGraph;

pub fn run<A: crate::Application + 'static>(app: A, graph: RenderGraph) {
    #[cfg(not(target_arch = "wasm32"))]
    crate::platform::winit::WinitPlatform::run(app, graph);
}
