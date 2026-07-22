use std::sync::Arc;

use wgpu::util::DeviceExt;

use crate::{
    assets::{AssetError, AssetLoader, SourceFor, graph::Node},
    renderer::{Mesh, MeshDefinition},
};

pub(crate) struct MeshInstance {
    pub(crate) index_buffer: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) vertex_buffer: wgpu::Buffer,
}

pub(crate) struct MeshLoader {
    device: Arc<wgpu::Device>,
}

impl MeshLoader {
    pub(crate) fn new(device: Arc<wgpu::Device>) -> Self {
        Self { device }
    }
}

impl AssetLoader<Mesh, MeshDefinition> for MeshLoader {
    type Built = MeshInstance;
    type Parsed = <MeshDefinition as SourceFor<Mesh>>::Raw;

    fn build(&self, src: &MeshDefinition, _: Self::Parsed) -> Result<Self::Built, AssetError> {
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Vertex Buffer"),
                contents: bytemuck::cast_slice(&src.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Index Buffer"),
                contents: bytemuck::cast_slice(&src.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Ok(MeshInstance {
            index_buffer,
            index_count: src.indices.len() as u32,
            vertex_buffer,
        })
    }

    fn parse(
        &self,
        _: &MeshDefinition,
        _: <MeshDefinition as SourceFor<Mesh>>::Raw,
    ) -> Result<(Self::Parsed, Option<Vec<Node>>), AssetError> {
        Ok(((), None))
    }
}
