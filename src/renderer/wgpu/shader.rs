use std::{borrow::Cow, sync::Arc};

use bytes::Bytes;

use crate::{
    assets::{AssetError, BuiltAsset, Loader},
    renderer::{
        BindGroupLayout, BindingType, Shader, ShaderDefinition, ShaderStage,
    },
};

impl BuiltAsset for Shader {
    type Built = ShaderInstance;
}

pub(crate) struct ShaderInstance {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) module: wgpu::ShaderModule,
}

pub(crate) struct ShaderLoader {
    device: Arc<wgpu::Device>,
}

impl ShaderLoader {
    pub(crate) fn new(device: Arc<wgpu::Device>) -> Self {
        Self { device }
    }
}

impl Loader<Shader> for ShaderLoader {
    type Desc = ShaderDefinition;
    type Parsed = Cow<'static, str>;

    fn build(
        &self,
        desc: &ShaderDefinition,
        parsed: Self::Parsed,
    ) -> Result<ShaderInstance, AssetError> {
        let module =
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(todo!()),
                    source: wgpu::ShaderSource::Wgsl(parsed),
                });

        Ok(ShaderInstance {
            layout: desc.layout.to_wgpu(self.device.as_ref()),
            module,
        })
    }

    fn parse(
        &self,
        _: &ShaderDefinition,
        bytes: Option<Bytes>,
    ) -> Result<Self::Parsed, AssetError> {
        if let Some(b) = bytes {
            return Ok(b.into());
        }

        Err(AssetError::InvalidArgument {
            msg: "expected bytes",
        })
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
                    ShaderStage::VertexFragment => {
                        wgpu::ShaderStages::VERTEX_FRAGMENT
                    }
                };

                let ty = match entry.ty {
                    BindingType::StorageBuffer {
                        has_dynamic_offset,
                        min_binding_size,
                    } => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset,
                        min_binding_size,
                    },
                    BindingType::Texture2D => wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true,
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    BindingType::TextureCube => wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true,
                        },
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
            })
            .collect::<Vec<_>>();

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Shader Bind Group Layout"),
            entries: &entries,
        })
    }
}
