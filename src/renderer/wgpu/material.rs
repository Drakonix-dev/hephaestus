use crate::{assets::AssetHandle, renderer::Shader};

pub(crate) struct MaterialInstance {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) shader: AssetHandle<Shader>,
}

pub(crate) struct MaterialLoader {}
