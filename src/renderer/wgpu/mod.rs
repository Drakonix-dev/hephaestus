mod material;
mod shader;
mod texture;

use std::sync::Arc;

use crate::{
    assets::AssetManager,
    renderer::wgpu::{shader::ShaderLoader, texture::TextureLoader},
};

fn register_loaders(mgr: &mut AssetManager, device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) {
    mgr.register(ShaderLoader::new(device.clone()));
    mgr.register(TextureLoader::new(device.clone(), queue.clone()));
}
