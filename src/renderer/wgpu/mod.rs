mod material;
mod mesh;
mod pipeline;
mod shader;
mod texture;

use std::sync::Arc;

use crate::{
    assets::AssetManager,
    renderer::wgpu::{
        material::MaterialLoader, mesh::MeshLoader, pipeline::PipelineLoader, shader::ShaderLoader,
        texture::TextureLoader,
    },
};

fn register_loaders(mgr: &mut AssetManager, device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) {
    mgr.register(MaterialLoader::new(device.clone()));
    mgr.register(MeshLoader::new(device.clone()));
    mgr.register(PipelineLoader::new(device.clone()));
    mgr.register(ShaderLoader::new(device.clone()));
    mgr.register(TextureLoader::new(device.clone(), queue.clone()));
}
