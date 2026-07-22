mod shader;
mod texture;

use std::sync::Arc;

use crate::{assets::AssetManager, renderer::wgpu::shader::ShaderLoader};

fn register_loaders(device: Arc<wgpu::Device>, mgr: &mut AssetManager) {
    mgr.register(ShaderLoader::new(device.clone()));
}

