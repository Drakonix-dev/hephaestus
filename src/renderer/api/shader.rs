use std::{num::NonZeroU64, path::PathBuf};

use crate::handles::define_handle;

// ShaderHandle defines a handle for a specific shader.
define_handle!(ShaderHandle);

pub struct ShaderDefinition {
    pub layout: BindGroupLayout,
    pub source: PathBuf,
}

pub struct BindGroupLayout {
    pub entries: Vec<BindingDesc>,
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
