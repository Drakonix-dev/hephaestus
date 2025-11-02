use bytemuck::{Pod, Zeroable};

use crate::{define_handle, define_wrapper};

// MeshHandle defines a handle for a specific mesh.
define_handle!(MeshHandle);

define_wrapper!(MeshManager, MeshBackend, {
    fn create_mesh(&mut self, def: &MeshDefinition) -> MeshHandle;
});

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
