use std::{collections::HashMap, fs, path::Path};

use crate::rendering::{ShaderDefinition, ShaderHandle};

pub(crate) struct ShaderManager {
    shaders: HashMap<ShaderHandle, ShaderInstance>,
    next_id: ShaderHandle,
}

pub(crate) struct ShaderInstance {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) module: wgpu::ShaderModule,
}

impl ShaderManager {
    pub(crate) fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            next_id: ShaderHandle::new(),
        }
    }

    pub(crate) fn create_shader(&mut self, device: &wgpu::Device, def: &ShaderDefinition) -> ShaderHandle{
        let source = fs::read_to_string(def.source)
            .unwrap_or_else(|_| panic!("failed to read shader file at {:?}", def.source));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
           label: Some(def.source.to_str().unwrap()),
           source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let handle = self.next_id;
        self.next_id = self.next_id.next();

        self.shaders.insert(handle, ShaderInstance {
            module: shader,
        });

        handle
    }

    pub(crate) fn get_shader(&self, handle: &ShaderHandle) -> Option<&ShaderInstance> {
        self.shaders.get(handle)
    }
}
