use std::{borrow::Cow, collections::HashMap, fs, sync::Arc};

use crate::{
    assets::{AssetError, AssetLoader, SourceFor},
    renderer::{
        BindGroupLayout, BindingType, RenderError, ShaderDefinition, ShaderHandle, ShaderStage,
        api::Shader,
    },
};

pub(crate) struct ShaderInstanceV2 {
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

impl AssetLoader<Shader, ShaderDefinition> for ShaderLoader {
    type Built = ShaderInstanceV2;
    type Parsed = <ShaderDefinition as SourceFor<Shader>>::Raw;

    fn parse(&self, raw: Self::Parsed) -> Result<Self::Parsed, AssetError> {
        Ok(raw)
    }

    fn build(&self, parsed: Self::Parsed) -> Result<Self::Built, AssetError> {
        let cow = String::from_utf8(parsed.2).expect("Invalid shader file");

        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(parsed.0.to_str().unwrap_or("unknown_shader")),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(cow)),
            });

        Ok(ShaderInstanceV2 {
            layout: parsed.1.to_wgpu(self.device.as_ref()),
            module,
        })
    }
}

pub(crate) struct ShaderInstance {
    pub(crate) handle: ShaderHandle,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) module: wgpu::ShaderModule,
}

pub(crate) struct ShaderManager {
    shaders: HashMap<ShaderHandle, ShaderInstance>,
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
    ) -> Result<(), RenderError> {
        let source =
            fs::read_to_string(def.source.as_path()).map_err(|err| RenderError::AssetLoad {
                path: def.source.clone(),
                source: Box::new(err),
            })?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(def.source.to_str().ok_or_else(|| RenderError::BadAsset {
                detail: format!("shader path is not valid UTF-8: {}", def.source.display()),
            })?),
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
