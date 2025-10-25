use std::collections::HashMap;

use crate::{renderer::wgpu::shader::ShaderInstance, rendering::{ShaderHandle, Vertex}};

pub(crate) struct PipelineManager {
    pipelines: HashMap<ShaderHandle, PipelineInstance>,
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

    fn create_pipeline(&mut self, device: &wgpu::Device, shader: &ShaderInstance) -> PipelineInstance {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&shader.layout],
            push_constant_ranges: &[],
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex> as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3, // position
                1 => Float32x3, // normal
                2 => Float32x2, // uv
            ],
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
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        PipelineInstance { pipeline }
    }

    pub(crate) fn get_or_create_pipeline(&mut self, device: &wgpu::Device, shader: &ShaderInstance) -> &PipelineInstance {
        if  !self.pipelines.contains_key(&shader.handle) {
            let pipeline = self.create_pipeline(device, shader);
            self.pipelines.insert(shader.handle, pipeline);
        }
        
        self.pipelines.get(&shader.handle).unwrap()
    }
}
