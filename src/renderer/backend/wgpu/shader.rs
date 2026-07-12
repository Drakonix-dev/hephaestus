use std::{collections::HashMap, fs};

use crate::{
    platform::core::PlatformError,
    renderer::{BindGroupLayout, BindingType, ShaderDefinition, ShaderHandle, ShaderStage},
};

pub(crate) struct ShaderManager {
    shaders: HashMap<ShaderHandle, ShaderInstance>,
}

pub(crate) struct ShaderInstance {
    pub(crate) handle: ShaderHandle,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) module: wgpu::ShaderModule,
}

impl ShaderManager {
    pub(crate) fn new() -> Self {
        Self {
            shaders: HashMap::new(),
        }
    }

    pub(crate) fn create_shader(
        &mut self,
        device: &wgpu::Device,
        handle: ShaderHandle,
        def: &ShaderDefinition,
    ) -> Result<(), PlatformError> {
        let source = fs::read_to_string(def.source.as_path())?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(
                def.source
                    .to_str()
                    .ok_or(PlatformError::AssetLoadFailed(def.source.clone()))?,
            ),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        self.shaders.insert(
            handle,
            ShaderInstance {
                handle,
                layout: def.layout.to_wgpu(device),
                module: shader,
            },
        );

        Ok(())
    }

    pub(crate) fn get_shader(&self, handle: &ShaderHandle) -> Option<&ShaderInstance> {
        self.shaders.get(handle)
    }
}

impl BindGroupLayout {
    fn to_wgpu(&self, device: &wgpu::Device) -> wgpu::BindGroupLayout {
        let entries = self
            .entries
            .iter()
            .map(|entry| {
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
                    BindingType::StorageBuffer {
                        has_dynamic_offset,
                        min_binding_size,
                    } => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset,
                        min_binding_size,
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
                    BindingType::UniformBuffer {
                        has_dynamic_offset,
                        min_binding_size,
                    } => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset,
                        min_binding_size,
                    },
                    BindingType::Sampler => {
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                    }
                };

                wgpu::BindGroupLayoutEntry {
                    binding: entry.binding,
                    visibility,
                    ty,
                    count: None,
                }
            })
            .collect::<Vec<_>>();

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shader Bind Group Layout"),
            entries: &entries,
        })
    }
}
