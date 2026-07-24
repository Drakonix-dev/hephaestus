use std::sync::Arc;

use crate::{
    assets::{AssetError, BuiltAs, Deps, Fetch, Handle, Loader, Priority, SourceFor},
    renderer::{Material, MaterialDefinition, Shader},
};

impl BuiltAs for Material {
    type Built = MaterialInstance;
}

pub(crate) struct MaterialInstance {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) shader: Handle<Shader>,
}

pub(crate) struct MaterialLoader {
    device: Arc<wgpu::Device>,
}

impl MaterialLoader {
    pub(crate) fn new(device: Arc<wgpu::Device>) -> Self {
        Self { device }
    }
}

impl Loader<Material, MaterialDefinition> for MaterialLoader {
    type Parsed = <MaterialDefinition as SourceFor<Material>>::Raw;

    fn build(
        &self,
        src: &MaterialDefinition,
        _: Self::Parsed,
        _: &Fetch,
    ) -> Result<MaterialInstance, AssetError> {
        todo!("still need to implement this")
    }

    fn parse(
        &self,
        src: &MaterialDefinition,
        _: <MaterialDefinition as SourceFor<Material>>::Raw,
        deps: &mut Deps,
    ) -> Result<Self::Parsed, AssetError> {
        deps.require_handle(src.shader, Priority::Critical);

        src.textures.iter().for_each(|h| {
            deps.require_handle(*h, Priority::Critical);
        });

        Ok(())
    }
}
