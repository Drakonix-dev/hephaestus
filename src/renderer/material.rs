use bytemuck::{Pod, Zeroable};

use crate::{
    asset_source,
    assets::{Asset, Handle},
    renderer::{Shader, Texture},
};

pub struct Material;
impl Asset for Material {}

pub struct MaterialDefinition {
    pub shader: Handle<Shader>,
    pub textures: Vec<Handle<Texture>>,
    pub uniforms: MaterialUniforms,
}

asset_source!(MaterialDefinition, { todo!() });

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct MaterialUniforms {
    pub base_color: [f32; 4],
}
