use crate::{core::math::{Transform2D, Transform3D}, define_handle, rendering::MaterialHandle};

// DrawMesh defines a command for drawing a single mesh.
pub struct DrawMesh {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Transform,
}

// MeshHandle defines a handle for a specific mesh.
define_handle!(MeshHandle);

// Transform defines a transformation.
#[derive(Debug, Clone, Copy)]
pub enum Transform {
    Transform2D(Transform2D),
    Transform3D(Transform3D),
}
