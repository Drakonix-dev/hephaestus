mod api;
mod platform;

pub(crate) mod macros;

pub mod events;
pub mod math;
pub mod renderer;

pub use api::*;

use crate::renderer::RenderGraph;

pub fn run<A: crate::Application + 'static>(
    app: A,
    graph: RenderGraph,
) -> Result<(), api::EngineError> {
    #[cfg(not(target_arch = "wasm32"))]
    Ok(crate::platform::winit::WinitPlatform::run(app, graph)?)
}
