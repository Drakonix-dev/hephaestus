use std::{num::NonZeroU64, path::Path};

use crate::{builtin::BuiltinShader, define_handle, define_wrapper};

// ShaderHandle defines a handle for a specific shader.
define_handle!(ShaderHandle);

define_wrapper!(ShaderManager, ShaderBackend, {
    fn builtin_shader(&self, shader: &BuiltinShader) -> ShaderHandle;
    fn create_shader(&mut self, def: &ShaderDefinition) -> ShaderHandle;
});

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
    StorageBuffer {
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZeroU64>,
    },
    Texture2D,
    TextureCube,
    UniformBuffer {
        has_dynamic_offset: bool,
        min_binding_size: Option<NonZeroU64>,
    },
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
