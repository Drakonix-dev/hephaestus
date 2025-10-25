use std::{collections::HashMap, fs};

use crate::rendering::{BindGroupLayout, BindingType, ShaderDefinition, ShaderHandle, ShaderStage};

pub(crate) struct ShaderManager {
    shaders: HashMap<ShaderHandle, ShaderInstance>,
    next_id: ShaderHandle,
}

pub(crate) struct ShaderInstance {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) module: wgpu::ShaderModule,
}

impl ShaderManager {
    pub(crate) fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            next_id: ShaderHandle::new(),
        }
    }

    pub(crate) fn create_shader(&mut self, device: &wgpu::Device, def: &ShaderDefinition) -> ShaderHandle{
        let source = fs::read_to_string(def.source)
            .unwrap_or_else(|_| panic!("failed to read shader file at {:?}", def.source));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
           label: Some(def.source.to_str().unwrap()),
           source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let handle = self.next_id;
        self.next_id = self.next_id.next();

        self.shaders.insert(handle, ShaderInstance {
            layout: def.layout.to_wgpu(&device),
            module: shader,
        });

        handle
    }

    pub(crate) fn get_shader(&self, handle: &ShaderHandle) -> Option<&ShaderInstance> {
        self.shaders.get(handle)
    }
}

impl<'a> BindGroupLayout<'a> {
    fn to_wgpu(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let entries = self.entries.iter().map(|entry| {
            let visibility = match entry.visibility {
                ShaderStage::Compute => wgpu::ShaderStages::COMPUTE,
                ShaderStage::Fragment => wgpu::ShaderStages::FRAGMENT,
                ShaderStage::Mesh => wgpu::ShaderStages::MESH,
                ShaderStage::None => wgpu::ShaderStages::NONE,
                ShaderStage::Task => wgpu::ShaderStages::TASK,
                ShaderStage::Vertex => wgpu::ShaderStages::VERTEX,
                ShaderStage::VertexFragment => wgpu::ShaderStages::VERTEX_FRAGMENT,
            };
            
            let ty = match entry.ty {
                BindingType::StorageBuffer => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                BindingType::Texture2D => wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                BindingType::TextureCube => wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                BindingType::UniformBuffer => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                BindingType::Sampler => wgpu::BindingType::Sampler(
                    wgpu::SamplerBindingType::Filtering,
                ),
            };

            wgpu::BindGroupLayoutEntry {
                binding: entry.binding,
                visibility,
                ty,
                count: None,
            }
        }).collect::<Vec<_>>();

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shader Bind Group Layout"),
            entries: &entries,
        })
    }
}
