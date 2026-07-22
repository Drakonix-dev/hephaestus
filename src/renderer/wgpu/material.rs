use std::sync::Arc;

use crate::{
    assets::{AssetError, AssetHandle, AssetLoader, SourceFor, graph},
    renderer::{Material, MaterialDefinition, Shader},
};

pub(crate) struct MaterialInstance {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) shader: AssetHandle<Shader>,
}

pub(crate) struct MaterialLoader {
    device: Arc<wgpu::Device>,
}

impl MaterialLoader {
    pub(crate) fn new(device: Arc<wgpu::Device>) -> Self {
        Self { device }
    }
}

impl AssetLoader<Material, MaterialDefinition> for MaterialLoader {
    type Built = MaterialInstance;
    type Parsed = <MaterialDefinition as SourceFor<Material>>::Raw;

    fn build(&self, src: &MaterialDefinition, _: Self::Parsed) -> Result<Self::Built, AssetError> {
        todo!("still need to implement this")
    }

    fn parse(
        &self,
        src: &MaterialDefinition,
        _: <MaterialDefinition as SourceFor<Material>>::Raw,
    ) -> Result<(Self::Parsed, Option<Vec<graph::Node>>), AssetError> {
        let mut deps = Vec::new();
        deps.push(src.shader.into());

        for h in src.textures.iter() {
            deps.push((*h).into());
        }

        Ok(((), Some(deps)))
    }
}
