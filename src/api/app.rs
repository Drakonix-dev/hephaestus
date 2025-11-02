use crate::{assets::{MaterialManager, MeshManager, ShaderManager, TextureManager}, core::{ecs::World, events::Event}, rendering::Renderer};

pub trait Application {
    fn init(&mut self, ctx: &ApplicationContext);
    fn handle_event(&mut self, ctx: &ApplicationContext, event: Event);
    fn update(&mut self, ctx: &ApplicationContext, dt: f32);
}

pub struct ApplicationContext<'a> {
    pub materials: &'a MaterialManager,
    pub meshes: &'a MeshManager,
    pub rendering: &'a Renderer,
    pub shaders: &'a ShaderManager,
    pub textures: &'a TextureManager,
    pub world: &'a mut World,
}
