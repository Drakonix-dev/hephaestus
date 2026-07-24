use bytemuck::{Pod, Zeroable};

use crate::{
    assets::{Asset, AssetError, Handle, SourceFor},
    renderer::{Shader, Texture},
};

pub struct Material;
impl Asset for Material {}

pub struct MaterialDefinition {
    pub shader: Handle<Shader>,
    pub textures: Vec<Handle<Texture>>,
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
