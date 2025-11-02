use bytemuck::{Pod, Zeroable};

use crate::{define_handle};

// MeshHandle defines a handle for a specific mesh.
define_handle!(MeshHandle);

pub struct MeshDefinition {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[doc(hidden)]
macro_rules! define_mesh {
    (
        $name:ident,
        $vertices:expr,
        $indices:expr
    ) => {
        impl MeshDefinition {
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

define_mesh!(triangle, vec![
    Vertex { position: [0.0, 0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.5, 0.0] },
    Vertex { position: [-0.5, -0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
    Vertex { position: [0.5, -0.5, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
], vec![0, 1, 2]);
