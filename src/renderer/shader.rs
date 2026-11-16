use std::{fs, num::NonZeroU64, path::PathBuf};

use crate::assets::{Asset, AssetError, Source, SourceId};

pub struct Shader;
impl Asset for Shader {}

pub struct ShaderDefinition {
    pub layout: BindGroupLayout,
    pub source: PathBuf,
}

impl Source for ShaderDefinition {
    type Output = Vec<u8>;

    fn id(&self) -> SourceId {
        todo!()
    }

    fn read(&self) -> Result<Self::Output, AssetError> {
        fs::read(self.source.as_path()).map_err(|err| err.into())
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
