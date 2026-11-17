use bytemuck::{Pod, Zeroable};

use crate::{
    assets::{Asset, Descriptor, Handle},
    renderer::{Shader, Texture},
};

pub struct Material;
impl Asset for Material {}

pub struct MaterialDefinition {
    pub shader: Handle<Shader>,
    pub textures: Vec<Handle<Texture>>,
    pub uniforms: MaterialUniforms,
}

impl Descriptor for MaterialDefinition {}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct MaterialUniforms {
    pub base_color: [f32; 4],
}
