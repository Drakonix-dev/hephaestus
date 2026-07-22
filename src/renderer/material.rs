use bytemuck::{Pod, Zeroable};

use crate::{
    assets::{Asset, AssetError, AssetHandle, SourceFor},
    renderer::{Shader, Texture},
};

pub struct Material;
impl Asset for Material {}

pub struct MaterialDefinition {
    pub shader: AssetHandle<Shader>,
    pub textures: Vec<AssetHandle<Texture>>,
    pub uniforms: MaterialUniforms,
}

impl SourceFor<Material> for MaterialDefinition {
    type Raw = ();

    fn fetch(&self) -> Result<Self::Raw, AssetError> {
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub struct MaterialUniforms {
    pub base_color: [f32; 4],
}
