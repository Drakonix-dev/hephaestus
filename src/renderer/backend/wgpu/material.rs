use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::{
    platform::core::PlatformError,
    renderer::{
        MaterialDefinition, MaterialHandle, ShaderHandle,
        backend::wgpu::{shader::ShaderManager, texture::TextureManager},
    },
};

pub(crate) struct MaterialManager {
    materials: HashMap<MaterialHandle, MaterialInstance>,
}

pub(crate) struct MaterialInstance {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) shader: ShaderHandle,
}

impl MaterialManager {
    pub(crate) fn new() -> Self {
        Self {
            materials: HashMap::new(),
        }
    }

    pub(crate) fn create_material(
        &mut self,
        device: &wgpu::Device,
        shaders: &ShaderManager,
        textures: &TextureManager,
        handle: MaterialHandle,
        def: &MaterialDefinition,
    ) -> Result<(), PlatformError> {
        let shader = shaders
            .get_shader(&def.shader)
            .ok_or(PlatformError::AssetNotFound(def.shader.to_string()))?;

        let texture_views: Vec<&wgpu::TextureView> = def
            .params
            .textures
            .iter()
            .map(|h| {
                let tex = textures
                    .get_texture(h)
                    .ok_or(PlatformError::AssetNotFound(h.to_string()))?;
                Ok(&tex.view)
            })
            .collect::<Result<_, PlatformError>>()?;

        let uniform_data = bytemuck::bytes_of(&def.params.uniforms);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Uniform Buffer"),
            contents: uniform_data,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = &shader.layout;

        let sampler = if texture_views.is_empty() {
            None
        } else {
            Some(device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Material Sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }))
        };

        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }]
        .into_iter()
        .chain(
            texture_views
                .iter()
                .enumerate()
                .map(|(i, view)| wgpu::BindGroupEntry {
                    binding: (i + 1) as u32,
                    resource: wgpu::BindingResource::TextureView(view),
                }),
        )
        .collect::<Vec<_>>();

        if let Some(sampler) = &sampler {
            entries.push(wgpu::BindGroupEntry {
                binding: (texture_views.len() + 1) as u32,
                resource: wgpu::BindingResource::Sampler(sampler),
            });
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material Bind Group"),
            layout: bind_group_layout,
            entries: &entries,
        });

        self.materials.insert(
            handle,
            MaterialInstance {
                bind_group,
                shader: def.shader,
            },
        );

        Ok(())
    }

    pub(crate) fn get_material(&self, handle: &MaterialHandle) -> Option<&MaterialInstance> {
        self.materials.get(handle)
    }
}
