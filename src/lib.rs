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

pub fn run_instance<A: crate::ApplicationInstance>(
    factory: impl Fn(&ApplicationContext) -> A + 'static,
    graph: RenderGraph,
) -> Result<(), EngineError> {
    let app = InstanceApplication::new(Box::new(factory));

    #[cfg(not(target_arch = "wasm32"))]
    Ok(crate::platform::winit::WinitPlatform::run(app, graph)?)
}

struct InstanceApplication<A: crate::ApplicationInstance> {
    factory: Box<dyn Fn(&ApplicationContext) -> A>,
}

impl<A: crate::ApplicationInstance> InstanceApplication<A> {
    fn new(factory: Box<dyn Fn(&ApplicationContext) -> A>) -> Self {
        Self { factory }
    }
}

impl<A: crate::ApplicationInstance> crate::Application for InstanceApplication<A> {
    type Instance = A;

    fn create(&mut self, ctx: &ApplicationContext) -> Self::Instance {
        (self.factory)(ctx)
    }
}
