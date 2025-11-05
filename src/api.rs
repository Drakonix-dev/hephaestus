use crate::{events::Event, renderer::RendererHandle};

pub trait Application {
    fn init(&mut self, ctx: &ApplicationContext);
    fn handle_event(&mut self, ctx: &ApplicationContext, event: Event);
    fn update(&mut self, ctx: &ApplicationContext, dt: f32);
}

pub struct ApplicationContext<'a> {
    pub renderer: &'a mut RendererHandle
}
