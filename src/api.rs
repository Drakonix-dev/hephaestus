use crate::{events::Event, renderer::{DrawCommand, RenderPhase, RendererHandle}};

pub trait Application {
    fn init(&mut self, ctx: &ApplicationContext);
    fn handle_event(&mut self, ctx: &ApplicationContext, event: Event);
    fn update(&mut self, _ctx: &ApplicationContext, _dt: f32) {}
    fn quit(&mut self) {}
    fn render(&mut self, _phase: &RenderPhase) -> Option<Vec<DrawCommand>> { None }
}

pub struct ApplicationContext<'a> {
    pub renderer: &'a mut RendererHandle
}
