use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::{
    assets::{AssetError, BuiltAs, DependencyKind, Deps, Fetch, Handle, Loader, SourceFor},
    renderer::{Material, MaterialDefinition, Shader},
};

impl BuiltAs for Material {
    type Built = MaterialInstance;
}

pub(crate) struct MaterialInstance {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) shader: Handle<Shader>,
}

pub(crate) struct MaterialLoader {
    device: Arc<wgpu::Device>,
}

impl MaterialLoader {
    pub(crate) fn new(device: Arc<wgpu::Device>) -> Self {
        Self { device }
    }
}

impl Loader<Material, MaterialDefinition> for MaterialLoader {
    type Parsed = ();

    fn build(
        &self,
        src: &MaterialDefinition,
        _: Self::Parsed,
        fetch: &Fetch,
    ) -> Result<MaterialInstance, AssetError> {
        let shader = fetch
            .get_handle(src.shader)?
            .ok_or(src.shader.not_ready())?;

        let texture_views: Vec<&wgpu::TextureView> = src
            .textures
            .iter()
            .map(|h| {
                let tex = fetch.get_handle(*h)?.ok_or(h.not_ready())?;
                Ok(&tex.view)
            })
            .collect::<Result<_, AssetError>>()?;

        let uniform_data = bytemuck::bytes_of(&src.uniforms);
        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Material Uniform Buffer"),
                contents: uniform_data,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

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

        let sampler = if texture_views.is_empty() {
            None
        } else {
            Some(self.device.create_sampler(&wgpu::SamplerDescriptor {
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

        if let Some(sampler) = &sampler {
            entries.push(wgpu::BindGroupEntry {
                binding: (texture_views.len() + 1) as u32,
                resource: wgpu::BindingResource::Sampler(sampler),
            });
        }

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material Bind Group"),
            layout: &shader.layout,
            entries: &entries,
        });

        Ok(MaterialInstance {
            bind_group,
            shader: src.shader,
        })
    }

    fn parse(
        &self,
        src: &MaterialDefinition,
        _: <MaterialDefinition as SourceFor<Material>>::Raw,
        deps: &mut Deps,
    ) -> Result<Self::Parsed, AssetError> {
        deps.require_handle(src.shader, DependencyKind::Required);

        src.textures.iter().for_each(|h| {
            deps.require_handle(*h, DependencyKind::Required);
        });

        Ok(())
    }
}
