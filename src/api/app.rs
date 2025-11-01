use crate::{assets::AssetManager, core::{ecs::World, events::Event}};

pub trait Application {
    fn init(&mut self, ctx: ApplicationContext, world: &mut World);
    fn handle_event(&mut self, world: &mut World, event: Event);
    fn update(&mut self, world: &mut World, dt: f32);
}

pub struct ApplicationContext {
    pub assets: AssetManager,
}
