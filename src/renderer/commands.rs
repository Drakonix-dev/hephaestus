use crate::{
    assets::AssetHandle,
    channels::define_channel,
    math::{Mat4, Transform2D, Transform3D},
    renderer::{Material, Mesh, RenderPhase},
};

pub(crate) enum RenderCommand {
    Draw(RenderPhase, DrawCommand),
    Render(Box<dyn Renderable + Send>),
    SetCamera(Mat4),
}

define_channel!(RenderQueue, RenderCommand);

pub trait Renderable {
    fn draw(&self, phase: RenderPhase) -> Vec<DrawCommand>;
}

pub enum DrawCommand {
    Mesh(DrawMesh),
}

pub struct DrawMesh {
    pub mesh: AssetHandle<Mesh>,
    pub material: AssetHandle<Material>,
    pub transform: Transform,
}

#[derive(Debug, Clone, Copy)]
pub enum Transform {
    Transform2D(Transform2D),
    Transform3D(Transform3D),
}
