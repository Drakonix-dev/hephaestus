use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::renderer::{backend::wgpu::{shader::ShaderManager, texture::TextureManager}, MaterialDefinition, MaterialHandle, ShaderHandle};

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
    ) {
        let shader = shaders.get_shader(&def.shader).
            expect("invalid shader");

        let texture_views: Vec<&wgpu::TextureView> = def.params.textures.iter()
            .map(|h| {
              &textures.get_texture(h)
                  .expect("invalid texture")
                  .view
            })
            .collect();

        let uniform_data = bytemuck::bytes_of(&def.params.uniforms);
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
           label: Some("Material Uniform Buffer"),
           contents: uniform_data,
           usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = &shader.layout;

        let entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
        ]
        .into_iter()
        .chain(texture_views.iter().enumerate().map(|(i, view)| {
            wgpu::BindGroupEntry {
                binding: (i + 1) as u32,
                resource: wgpu::BindingResource::TextureView(view),
            }
        }))
        .collect::<Vec<_>>();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
           label: Some("Material Bind Group"),
           layout: bind_group_layout,
           entries: &entries,
        });

        self.materials.insert(handle, MaterialInstance {
            bind_group,
            shader: def.shader,
        });
    }

    pub(crate) fn get_material(&self, handle: &MaterialHandle) -> Option<&MaterialInstance> {
        self.materials.get(handle)
    }
}
