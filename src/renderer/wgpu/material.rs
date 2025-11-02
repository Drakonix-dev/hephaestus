use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::{assets::{MaterialDefinition, MaterialHandle, ShaderHandle}, renderer::wgpu::{shader::ShaderManager, texture::TextureManager, Surface}};

pub(crate) struct MaterialManager<'a> {
    materials: HashMap<MaterialHandle, MaterialInstance>,
    next_id: MaterialHandle,
    surface: &'a Surface<'a>,
}

pub(crate) struct MaterialInstance {
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) shader: ShaderHandle,
}

impl<'a> MaterialManager<'a> {
    pub(crate) fn new(surface: &'a Surface<'a>) -> Self {
        Self {
            materials: HashMap::new(),
            next_id: MaterialHandle::new(),
            surface,
        }
    }

    pub(crate) fn create_material(
        &mut self,
        device: &wgpu::Device,
        shaders: &ShaderManager,
        textures: &TextureManager,
        def: &MaterialDefinition,
    ) -> MaterialHandle {
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

        let handle = self.next_id;
        self.next_id = self.next_id.next();

        self.materials.insert(handle, MaterialInstance {
            bind_group,
            shader: def.shader,
        });

        handle
    }

    pub(crate) fn get_material(&self, handle: &MaterialHandle) -> Option<&MaterialInstance> {
        self.materials.get(handle)
    }
}
