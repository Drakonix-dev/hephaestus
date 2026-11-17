use std::num::NonZeroU64;

use crate::assets::{Asset, AssetData, Descriptor};

pub struct Shader;
impl Asset for Shader {}

pub struct ShaderDefinition {
    pub layout: BindGroupLayout,
    pub source: Option<AssetData>,
}

impl Descriptor for ShaderDefinition {
    fn take_external(&mut self) -> Option<AssetData> {
        self.source.take()
    }
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
