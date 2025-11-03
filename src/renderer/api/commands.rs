use crate::{core::math::{Transform2D, Transform3D}, renderer::{MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle, RenderPhase, ShaderDefinition, ShaderHandle, TextureDefinition, TextureHandle}};

// RenderCommand defines a command for rendering.
pub(crate) enum RenderCommand {
    CreateMaterial(MaterialHandle, MaterialDefinition),
    CreateMesh(MeshHandle, MeshDefinition),
    CreateShader(ShaderHandle, ShaderDefinition),
    CreateTexture(TextureHandle, TextureDefinition),
    Draw(RenderPhase, DrawCommand),
    Register(Box<dyn Renderable + Send>),
}

// DrawCommand defines a command for drawing something to the window.
pub enum DrawCommand {
    Mesh(DrawMesh),
}

// DrawMesh defines a command for drawing a single mesh.
pub struct DrawMesh {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Transform,
}

// Renderable defines something that can be rendered.
pub trait Renderable {
    fn draw(&self, phase: RenderPhase) -> Vec<DrawCommand>;   
}

// Transform defines a transformation.
#[derive(Debug, Clone, Copy)]
pub enum Transform {
    Transform2D(Transform2D),
    Transform3D(Transform3D),
}
