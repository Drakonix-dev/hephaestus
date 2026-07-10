use std::collections::HashMap;

use wgpu::util::DeviceExt;

use crate::renderer::{MeshDefinition, MeshHandle};

pub(crate) struct MeshManager {
    meshes: HashMap<MeshHandle, MeshInstance>,
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
        }
    }
    
    pub(crate) fn create_mesh(
        &mut self,
        device: &wgpu::Device,
        handle: MeshHandle,
        def: &MeshDefinition,
    ) {
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

        self.meshes.insert(handle, MeshInstance {
            index_buffer,
            index_count: def.indices.len() as u32,
            vertex_buffer,
        });
    }

    pub(crate) fn get_mesh(&self, handle: &MeshHandle) -> Option<&MeshInstance> {
        self.meshes.get(handle)
    }
}
