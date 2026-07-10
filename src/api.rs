use core::time;
use std::{error::Error, sync::atomic::{AtomicBool, Ordering}};

use crate::{events::Event, renderer::{DrawCommand, RenderPhase, RendererHandle}};

pub trait Application {
    fn init(&mut self, ctx: &ApplicationContext);
    fn handle_error(&mut self, _err: impl Into<EngineError>) {}
    fn handle_event(&mut self, ctx: &ApplicationContext, event: Event);
    fn update(&mut self, _ctx: &ApplicationContext, _dt: time::Duration) {}
    fn quit(&mut self) {}
    fn render(&mut self, _phase: &RenderPhase) -> Option<Vec<DrawCommand>> { None }
}

pub struct ApplicationContext<'a> {
    exit_requested: AtomicBool,
    pub renderer: &'a mut RendererHandle
}

impl<'a> ApplicationContext<'a> {
    pub(crate) fn new(renderer: &'a mut RendererHandle) -> Self {
        Self {
            exit_requested: AtomicBool::new(false),
            renderer,
        }
    }
    
    pub fn exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::SeqCst)
    }
    
    pub fn request_exit(&self) {
        self.exit_requested.store(true, Ordering::SeqCst);
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
