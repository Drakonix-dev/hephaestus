use std::{fs, num::NonZeroU64, path::PathBuf};

use crate::assets::{Asset, AssetError, SourceFor};

pub struct Shader;
impl Asset for Shader {}

pub struct ShaderDefinition {
    pub layout: BindGroupLayout,
    pub source: PathBuf,
}

impl SourceFor<Shader> for ShaderDefinition {
    type Raw = (PathBuf, BindGroupLayout, Vec<u8>);

    fn fetch(&self) -> Result<Self::Raw, AssetError> {
        fs::read(self.source.as_path())
            .map(|v| (self.source, self.layout, v))
            .map_err(|err| AssetError::Other {
                source: Box::new(err),
            })
    }
}

#[derive(Clone)]
pub struct BindGroupLayout {
    pub entries: Vec<BindingDesc>,
}

#[derive(Clone)]
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
