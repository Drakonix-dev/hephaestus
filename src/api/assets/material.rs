use bytemuck::{Pod, Zeroable};

use crate::{assets::{ShaderHandle, TextureHandle}, define_handle, define_wrapper};

// MaterialHandle defines a handle for a specific material.
define_handle!(MaterialHandle);

define_wrapper!(MaterialManager, MaterialDevice, {
    fn create_material(&mut self, def: &MaterialDefinition) -> MaterialHandle;
});

#[derive(Clone)]
pub struct MaterialDefinition {
    pub shader: ShaderHandle,
    pub params: MaterialParams,
}

#[derive(Clone)]
pub struct MaterialParams {
    pub textures: Vec<TextureHandle>,
    pub uniforms: MaterialUniforms,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct MaterialUniforms {
    pub base_color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
}
