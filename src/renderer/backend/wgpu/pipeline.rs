use std::collections::HashMap;

use crate::renderer::{ShaderHandle, Vertex, backend::wgpu::shader::ShaderInstance};

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RenderState {
    pub(crate) depth_enabled: bool,
}

pub(crate) struct PipelineManager {
    pipelines: HashMap<(ShaderHandle, RenderState), PipelineInstance>,
}

pub(crate) struct PipelineInstance {
    pub(crate) pipeline: wgpu::RenderPipeline,
}

impl PipelineManager {
    pub(crate) fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    fn create_pipeline(
        &mut self,
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        shader: &ShaderInstance,
        globals_layout: &wgpu::BindGroupLayout,
        model_layout: &wgpu::BindGroupLayout,
        render_state: RenderState,
    ) -> PipelineInstance {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[globals_layout, &shader.layout, model_layout],
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

        let depth_stencil = if render_state.depth_enabled {
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            })
        } else {
            None
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader.module,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader.module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        PipelineInstance { pipeline }
    }

    pub(crate) fn get_or_create_pipeline(
        &mut self,
        format: wgpu::TextureFormat,
        device: &wgpu::Device,
        shader: &ShaderInstance,
        globals_layout: &wgpu::BindGroupLayout,
        model_layout: &wgpu::BindGroupLayout,
        render_state: RenderState,
    ) -> &PipelineInstance {
        let key = (shader.handle, render_state);

        if !self.pipelines.contains_key(&key) {
            let pipeline = self.create_pipeline(
                format,
                device,
                shader,
                globals_layout,
                model_layout,
                render_state,
            );
            self.pipelines.insert(key, pipeline);
        }

        self.pipelines.get(&key).unwrap()
    }
}
