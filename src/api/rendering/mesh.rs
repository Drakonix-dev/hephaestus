use bytemuck::{Pod, Zeroable};

use crate::{core::math::{Transform2D, Transform3D}, define_handle, rendering::MaterialHandle};

// DrawMesh defines a command for drawing a single mesh.
pub struct DrawMesh {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub transform: Transform,
}

// MeshHandle defines a handle for a specific mesh.
define_handle!(MeshHandle);

pub struct MeshDefinition<'a> {
    pub vertices: &'a [Vertex],
    pub indices: &'a [u32],
}

#[doc(hidden)]
macro_rules! define_mesh {
    (
        $name:ident,
        $vertices:expr,
        $indices:expr
    ) => {
        impl<'a> MeshDefinition<'a> {
            pub fn $name() -> Self {
                Self {
                    vertices: $vertices,
                    indices: $indices,
                }
            }
        }
    };
}

// Transform defines a transformation.
#[derive(Debug, Clone, Copy)]
pub enum Transform {
    Transform2D(Transform2D),
    Transform3D(Transform3D),
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

define_mesh!(triangle, &[
    Vertex { position: [0.0, 0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.5, 0.0] },
    Vertex { position: [-0.5, -0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
    Vertex { position: [0.5, -0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
], &[0, 1, 2]);
