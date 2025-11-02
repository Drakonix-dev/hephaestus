use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::{assets::{MeshDefinition, MeshDevice, MeshHandle}, renderer::wgpu::Surface};

pub(crate) struct MeshManager<'a> {
    meshes: HashMap<MeshHandle, MeshInstance>,
    next_id: MeshHandle,
    surface: &'a Surface<'a>,
}

pub(crate) struct MeshInstance {
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) vertex_buffer: wgpu::Buffer,
}

impl<'a> MeshManager<'a> {
    pub(crate) fn new(surface: &'a Surface<'a>) -> Self {
        Self {
            meshes: HashMap::new(),
            next_id: MeshHandle::new(),
            surface,
        }
    }

    pub(crate) fn get_mesh(&self, handle: &MeshHandle) -> Option<&MeshInstance> {
        self.meshes.get(handle)
    }
}

impl<'a> MeshDevice for MeshManager<'a> {
    fn create_mesh(&mut self, def: &MeshDefinition) -> MeshHandle {
        let vertex_buffer = self.surface.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
           label: Some("Mesh Vertex Buffer"),
           contents: bytemuck::cast_slice(&def.vertices),
           usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = self.surface.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
           label: Some("Mesh Index Buffer"),
           contents: bytemuck::cast_slice(&def.indices),
           usage: wgpu::BufferUsages::INDEX,
        });

        let handle = self.next_id;
        self.next_id = self.next_id.next();

        self.meshes.insert(handle, MeshInstance {
            index_buffer,
            index_count: def.indices.len() as u32,
            vertex_buffer,
        });

        handle
    }
}
