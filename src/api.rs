use core::time;
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use crate::{
    commands::{EngineCommand, EngineCommandWriter},
    config::WindowMode,
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
    exit_requested: &'a Cell<bool>,
    pub engine: &'a EngineHandle,
    pub renderer: &'a mut RendererHandle,
}

impl<'a> ApplicationContext<'a> {
    pub(crate) fn new(
        exit_requested: &'a Cell<bool>,
        renderer: &'a mut RendererHandle,
        engine: &'a EngineHandle,
    ) -> Self {
        Self {
            exit_requested,
            engine,
            renderer,
        }
    }

    pub fn request_exit(&self) {
        self.exit_requested.set(true);
    }
}

pub struct EngineHandle {
    commands: EngineCommandWriter,
    tick_rate_hz: Arc<AtomicU32>,
}

impl EngineHandle {
    pub(crate) fn new(commands: EngineCommandWriter, tick_rate_hz: Arc<AtomicU32>) -> Self {
        Self {
            commands,
            tick_rate_hz,
        }
    }

    pub fn set_present_mode(&self, mode: PresentMode) {
        self.commands.push(EngineCommand::SetPresentMode(mode));
    }

    pub fn set_tick_rate(&self, hz: u32) {
        self.tick_rate_hz.store(hz.max(1), Ordering::Relaxed);
    }

    pub fn set_window_mode(&self, mode: WindowMode) {
        self.commands.push(EngineCommand::SetWindowMode(mode));
    }

    pub fn tick_rate(&self) -> u32 {
        self.tick_rate_hz.load(Ordering::Relaxed)
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
