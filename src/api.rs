use core::time;

use crate::{
    commands::{EngineCommand, EngineCommandWriter},
    config::{RuntimeConfig, WindowMode},
    events::Event,
    platform::core::PlatformError,
    renderer::{DrawCommand, PresentMode, RenderError, RenderPhase, RendererHandle},
};

pub trait Application {
    type Instance: ApplicationInstance;

    fn create(&mut self, _ctx: &ApplicationContext) -> Self::Instance;
    fn handle_error(&mut self, _err: impl Into<EngineError>) {}
    fn quit(&mut self) {}
}

pub trait ApplicationInstance {
    fn handle_error(&mut self, _err: impl Into<EngineError>) {}
    fn handle_event(&mut self, _ctx: &ApplicationContext, _event: Event) {}
    fn update(&mut self, _ctx: &ApplicationContext, _dt: time::Duration) {}
    fn quit(&mut self) {}
    fn render(&mut self, _phase: &RenderPhase) -> Option<Vec<DrawCommand>> {
        None
    }
}

pub struct ApplicationContext<'a> {
    commands: &'a EngineCommandWriter,
    config: &'a RuntimeConfig,
    pub renderer: &'a mut RendererHandle,
}

impl<'a> ApplicationContext<'a> {
    pub(crate) fn new(
        commands: &'a EngineCommandWriter,
        config: &'a RuntimeConfig,
        renderer: &'a mut RendererHandle,
    ) -> Self {
        Self {
            commands,
            config,
            renderer,
        }
    }

    pub fn config(&self) -> &RuntimeConfig {
        self.config
    }

    pub fn request_exit(&self) {
        self.commands.push(EngineCommand::RequestExit);
    }

    pub fn set_present_mode(&self, mode: PresentMode) {
        self.commands.push(EngineCommand::SetPresentMode(mode));
    }

    pub fn set_tick_rate(&self, hz: u32) {
        self.commands.push(EngineCommand::SetTickRate(hz));
    }

    pub fn set_window_mode(&self, mode: WindowMode) {
        self.commands.push(EngineCommand::SetWindowMode(mode));
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
