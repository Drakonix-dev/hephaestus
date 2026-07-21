use std::time;

use crate::{
    assets::AssetManager,
    commands::{EngineCommand, EngineCommandWriter},
    config::RuntimeConfig,
    events::EventBus,
    platform::PlatformError,
    renderer::{DrawCommand, RenderError, RenderPhase, RendererHandle, Viewport},
};

pub trait Application {
    type Instance: ApplicationInstance;

    fn create(&mut self, _ctx: &ApplicationContext) -> Self::Instance;
    fn handle_error(&mut self, _err: impl Into<EngineError>) {}
    fn quit(&mut self) {}
}

pub trait ApplicationInstance {
    fn handle_error(&mut self, _err: impl Into<EngineError>) {}
    fn update(&mut self, _ctx: &ApplicationContext, _dt: time::Duration) {}
    fn quit(&mut self) {}
    fn render(&mut self, _phase: &RenderPhase, _alpha: f32) -> Option<Vec<DrawCommand>> {
        None
    }
}

pub struct ApplicationContext<'a> {
    pub assets: &'a mut AssetManager,
    pub(crate) commands: &'a EngineCommandWriter,
    pub config: &'a RuntimeConfig,
    pub events: &'a mut EventBus,
    pub renderer: &'a mut RendererHandle,
    pub viewport: &'a Viewport,
}

impl<'a> ApplicationContext<'a> {
    pub fn send_cmd(&self, cmd: EngineCommand) {
        self.commands.push(cmd);
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EngineError {
    #[error(transparent)]
    Platform(#[from] PlatformError),

    #[error(transparent)]
    Render(#[from] RenderError),
}
