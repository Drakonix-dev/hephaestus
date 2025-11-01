use crate::{builtin::BuiltinShader, rendering::{MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle, ShaderDefinition, ShaderHandle, TextureDefinition, TextureHandle}};

pub(crate) trait AssetManagerBackend {
    fn builtin_shader(&self, shader: &BuiltinShader) -> ShaderHandle;
    
    fn create_material(&mut self, def: &MaterialDefinition) -> MaterialHandle;
    fn create_mesh(&mut self, def: &MeshDefinition) -> MeshHandle;
    fn create_shader(&mut self, def: &ShaderDefinition) -> ShaderHandle;
    fn create_texture(&mut self, def: &TextureDefinition) -> TextureHandle;
}

pub struct AssetManager {
    backend: Box<dyn AssetManagerBackend>,
}

impl AssetManager {
    pub fn builtin_shader(&self, shader: &BuiltinShader) -> ShaderHandle {
        self.backend.builtin_shader(shader)
    }
    
    pub fn create_material(&mut self, def: &MaterialDefinition) -> MaterialHandle {
        self.backend.create_material(def)
    }

    pub fn create_mesh(&mut self, def: &MeshDefinition) -> MeshHandle {
        self.backend.create_mesh(def)
    }

    pub fn create_shader(&mut self, def: &ShaderDefinition) -> ShaderHandle {
        self.backend.create_shader(def)
    }

    pub fn create_texture(&mut self, def: &TextureDefinition) -> TextureHandle {
        self.backend.create_texture(def)
    }
}
