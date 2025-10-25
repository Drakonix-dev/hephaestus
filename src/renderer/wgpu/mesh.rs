use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::rendering::{MeshDefinition, MeshHandle};

pub(crate) struct MeshManager {
    meshes: HashMap<MeshHandle, MeshInstance>,
    next_id: MeshHandle,
}

pub(crate) struct MeshInstance {
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) vertex_buffer: wgpu::Buffer,
}

impl MeshManager {
    pub(crate) fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            next_id: MeshHandle::new(),
        }
    }

    pub(crate) fn create_mesh(&mut self, device: &wgpu::Device, def: &MeshDefinition) -> MeshHandle {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
           label: Some("Mesh Vertex Buffer"),
           contents: bytemuck::cast_slice(&def.vertices),
           usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

    pub(crate) fn get_mesh(&self, handle: &MeshHandle) -> Option<&MeshInstance> {
        self.meshes.get(handle)
    }
}
