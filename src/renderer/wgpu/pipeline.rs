use std::sync::Arc;

use crate::{
    assets::{Asset, AssetError, BuiltAs, Deps, Fetch, Handle, Loader, Priority, SourceFor},
    renderer::Shader,
};

pub(crate) struct Pipeline;
impl Asset for Pipeline {}

impl BuiltAs for Pipeline {
    type Built = PipelineInstance;
}

pub(crate) struct PipelineInstance {
    pub(crate) pipeline: wgpu::RenderPipeline,
}

pub(crate) struct RenderState {
    pub(crate) depth_enabled: bool,
}

pub(crate) struct PipelineDefinition {
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) globals_layout: Arc<wgpu::BindGroupLayout>,
    pub(crate) model_layout: Arc<wgpu::BindGroupLayout>,
    pub(crate) render_state: RenderState,
    pub(crate) shader: Handle<Shader>,
}

impl SourceFor<Pipeline> for PipelineDefinition {
    type Raw = ();

    fn fetch(&self) -> Result<Self::Raw, AssetError> {
        Ok(())
    }
}

pub(crate) struct PipelineLoader {
    device: Arc<wgpu::Device>,
}

impl PipelineLoader {
    pub(crate) fn new(device: Arc<wgpu::Device>) -> Self {
        Self { device }
    }
}

impl Loader<Pipeline, PipelineDefinition> for PipelineLoader {
    type Parsed = <PipelineDefinition as SourceFor<Pipeline>>::Raw;

    fn build(
        &self,
        _: &PipelineDefinition,
        _: Self::Parsed,
        _: &Fetch,
    ) -> Result<PipelineInstance, AssetError> {
        todo!("build the pipeline")
    }

    fn parse(
        &self,
        src: &PipelineDefinition,
        _: <PipelineDefinition as SourceFor<Pipeline>>::Raw,
        deps: &mut Deps,
    ) -> Result<Self::Parsed, AssetError> {
        deps.require_handle(src.shader, Priority::Critical);

        Ok(())
    }
}
