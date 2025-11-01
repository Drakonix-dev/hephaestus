use crate::{assets::{MaterialHandle, MeshHandle}, core::math::{Transform2D, Transform3D}};

// DrawCommand defines a command that can be translated to GPU calls.
pub enum DrawCommand {
    Mesh(DrawMesh),
}

// DrawMesh defines a command for drawing a single mesh.
pub struct DrawMesh {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Transform,
}

// Transform defines a transformation.
#[derive(Debug, Clone, Copy)]
pub enum Transform {
    Transform2D(Transform2D),
    Transform3D(Transform3D),
}
