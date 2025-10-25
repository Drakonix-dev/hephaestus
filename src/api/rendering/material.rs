use bytemuck::{Pod, Zeroable};

use crate::{define_handle, rendering::{ShaderHandle, TextureHandle}};

// MaterialHandle defines a handle for a specific material.
define_handle!(MaterialHandle);

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
