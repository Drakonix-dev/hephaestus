use std::sync::Arc;

use crate::{
    assets::{Asset, AssetError, BuiltAs, DependencyKind, Deps, Fetch, Handle, Loader, SourceFor},
    renderer::{Shader, Vertex},
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
    type Parsed = ();

    fn build(
        &self,
        src: &PipelineDefinition,
        _: Self::Parsed,
        fetch: &Fetch,
    ) -> Result<PipelineInstance, AssetError> {
        let shader = fetch
            .get_handle(src.shader)?
            .ok_or(src.shader.not_ready())?;
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[&src.globals_layout, &shader.layout, &src.model_layout],
                push_constant_ranges: &[],
            });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, // position
                1 => Float32x3, // normal
                2 => Float32x2, // uv
            ],
        };

        let depth_stencil = if src.render_state.depth_enabled {
            Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        };

        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    buffers: &[vertex_buffer_layout],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    entry_point: Some("vs_main"),
                    module: &shader.module,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader.module,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        blend: Some(wgpu::BlendState::REPLACE),
                        format: src.format,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        Ok(PipelineInstance { pipeline })
    }

    fn parse(
        &self,
        src: &PipelineDefinition,
        _: <PipelineDefinition as SourceFor<Pipeline>>::Raw,
        deps: &mut Deps,
    ) -> Result<Self::Parsed, AssetError> {
        deps.require_handle(src.shader, DependencyKind::Required);

        Ok(())
    }
}
