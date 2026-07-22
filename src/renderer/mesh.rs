use bytemuck::{Pod, Zeroable};

use crate::assets::{Asset, AssetError, SourceFor};

pub struct Mesh;
impl Asset for Mesh {}

pub struct MeshDefinition {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
}

impl SourceFor<Mesh> for MeshDefinition {
    type Raw = ();

    fn fetch(&self) -> Result<Self::Raw, AssetError> {
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

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

macro_rules! vertex {
    ($position:expr, $normal:expr, $uv:expr) => {
        Vertex {
            position: $position,
            normal: $normal,
            uv: $uv,
        }
    };
}

define_mesh!(
    triangle,
    vec![
        vertex!([0.0, 0.5, 0.0], [0.0, 0.0, 1.0], [0.5, 0.0]),
        vertex!([-0.5, -0.5, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
        vertex!([0.5, -0.5, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
    ],
    vec![0, 1, 2]
);

define_mesh!(
    cube,
    vec![
        // Front (+Z)
        vertex!([-0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 0.0]),
        vertex!([0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 0.0]),
        vertex!([0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 1.0]),
        vertex!([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 1.0]),
        // Back (-Z)
        vertex!([0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 0.0]),
        vertex!([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 0.0]),
        vertex!([-0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 1.0]),
        vertex!([0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 1.0]),
        // Right (+X)
        vertex!([0.5, -0.5, 0.5], [1.0, 0.0, 0.0], [0.0, 0.0]),
        vertex!([0.5, -0.5, -0.5], [1.0, 0.0, 0.0], [1.0, 0.0]),
        vertex!([0.5, 0.5, -0.5], [1.0, 0.0, 0.0], [1.0, 1.0]),
        vertex!([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], [0.0, 1.0]),
        // Left (-X)
        vertex!([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 0.0]),
        vertex!([-0.5, -0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 0.0]),
        vertex!([-0.5, 0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 1.0]),
        vertex!([-0.5, 0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 1.0]),
        // Top (+Y)
        vertex!([-0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
        vertex!([0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [1.0, 0.0]),
        vertex!([0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [1.0, 1.0]),
        vertex!([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 1.0]),
        // Bottom (-Y)
        vertex!([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [0.0, 0.0]),
        vertex!([0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [1.0, 0.0]),
        vertex!([0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [1.0, 1.0]),
        vertex!([-0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [0.0, 1.0]),
    ],
    vec![
        0, 1, 2, 0, 2, 3, // front
        4, 5, 6, 4, 6, 7, // back
        8, 9, 10, 8, 10, 11, // right
        12, 13, 14, 12, 14, 15, // left
        16, 17, 18, 16, 18, 19, // top
        20, 21, 22, 20, 22, 23, // bottom
    ]
);
