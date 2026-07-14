pub use api::{Application, ApplicationContext, ApplicationInstance, EngineError};
pub use platform::PlatformError;

pub mod buffers;
pub mod commands;
pub mod config;
pub mod diagnostics;
pub mod events;
pub mod math;
pub mod renderer;

pub(crate) mod channels;
pub(crate) mod handles;

mod api;
mod platform;

use crate::{config::EngineConfig, renderer::RenderGraph};

#[cfg(not(target_arch = "wasm32"))]
pub fn run<A: crate::Application + 'static>(
    app: A,
    cfg: EngineConfig,
    graph: RenderGraph,
) -> Result<(), api::EngineError> {
    Ok(crate::platform::winit::WinitPlatform::run(app, cfg, graph)?)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_instance<A: crate::ApplicationInstance>(
    factory: impl Fn(&ApplicationContext) -> A + 'static,
    cfg: EngineConfig,
    graph: RenderGraph,
) -> Result<(), EngineError> {
    let app = InstanceApplication::new(Box::new(factory));

    Ok(crate::platform::winit::WinitPlatform::run(app, cfg, graph)?)
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
