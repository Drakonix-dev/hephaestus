use std::sync::Arc;

use crate::{
    assets::{Asset, AssetError, AssetHandle, AssetLoader, SourceFor, graph::Node},
    renderer::Shader,
};

pub(crate) struct Pipeline;
impl Asset for Pipeline {}

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
    pub(crate) shader: AssetHandle<Shader>,
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

impl AssetLoader<Pipeline, PipelineDefinition> for PipelineLoader {
    type Built = PipelineInstance;
    type Parsed = <PipelineDefinition as SourceFor<Pipeline>>::Raw;

    fn build(&self, src: &PipelineDefinition, _: Self::Parsed) -> Result<Self::Built, AssetError> {
        todo!("build the pipeline")
    }

    fn parse(
        &self,
        src: &PipelineDefinition,
        _: <PipelineDefinition as SourceFor<Pipeline>>::Raw,
    ) -> Result<(Self::Parsed, Option<Vec<Node>>), AssetError> {
        let mut deps = Vec::new();
        deps.push(src.shader.into());

        Ok(((), Some(deps)))
    }
}
