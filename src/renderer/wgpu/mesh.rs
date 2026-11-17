use std::sync::Arc;

use bytes::Bytes;
use wgpu::util::DeviceExt;

use crate::{
    assets::{AssetError, BuiltAsset, Loader},
    renderer::{Mesh, MeshDefinition},
};

impl BuiltAsset for Mesh {
    type Built = MeshInstance;
}

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

impl Loader<Mesh> for MeshLoader {
    type Desc = MeshDefinition;
    type Parsed = ();

    fn build(
        &self,
        src: &MeshDefinition,
        _: Self::Parsed,
    ) -> Result<MeshInstance, AssetError> {
        let vertex_buffer =
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh Vertex Buffer"),
                    contents: bytemuck::cast_slice(&src.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let index_buffer =
            self.device
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
        _: Option<Bytes>,
    ) -> Result<Self::Parsed, AssetError> {
        Ok(())
    }
}
