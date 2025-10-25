use std::path::Path;

use crate::define_handle;

// ShaderHandle defines a handle for a specific shader.
define_handle!(ShaderHandle);

pub struct ShaderDefinition<'a> {
    pub layout: BindGroupLayout<'a>,
    pub source: &'a Path,
}

pub struct BindGroupLayout<'a> {
    pub entries: &'a [BindingDesc],
}

pub struct BindingDesc {
    pub binding: u32,
    pub ty: BindingType,
    pub visibility: ShaderStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingType {
    Sampler,
    StorageBuffer,
    Texture2D,
    TextureCube,
    UniformBuffer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    Compute,
    Fragment,
    Mesh,
    None,
    Task,
    Vertex,
    VertexFragment,
}
