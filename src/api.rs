use core::time;
use std::{cell::Cell, error::Error};

use crate::{
    events::Event,
    renderer::{DrawCommand, RenderPhase, RendererHandle},
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
    pub renderer: &'a mut RendererHandle,
}

impl<'a> ApplicationContext<'a> {
    pub(crate) fn new(exit_requested: &'a Cell<bool>, renderer: &'a mut RendererHandle) -> Self {
        Self {
            exit_requested,
            renderer,
        }
    }

    pub fn request_exit(&self) {
        self.exit_requested.set(true);
    }
}

#[derive(Debug)]
pub enum EngineError {
    Fatal {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
    Recoverable {
        message: String,
        source: Option<Box<dyn Error + Send + Sync>>,
    },
}
