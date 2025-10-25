use crate::{core::{ecs::World, events::Event}, rendering::Renderer};

pub trait Application {
    fn init(&mut self, context: &mut ApplicationContext);
    fn handle_event(&mut self, context: &mut ApplicationContext, event: Event);
}

pub struct ApplicationContext {
    pub renderer: Renderer,
    pub world: World,
}
