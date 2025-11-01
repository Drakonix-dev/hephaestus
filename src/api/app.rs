use crate::{assets::AssetManager, core::{ecs::World, events::Event}, rendering::Renderer};

pub trait Application {
    fn init(&mut self, ctx: &ApplicationContext);
    fn handle_event(&mut self, ctx: &ApplicationContext, event: Event);
    fn update(&mut self, ctx: &ApplicationContext, dt: f32);
}

pub struct ApplicationContext<'a> {
    pub assets: &'a AssetManager,
    pub rendering: &'a Renderer,
    pub world: &'a mut World,
}
