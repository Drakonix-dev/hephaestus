mod builtin;
mod commands;
mod graph;
mod material;
mod mesh;
mod shader;
mod texture;

pub use builtin::*;
pub use commands::*;
pub use graph::*;
pub use material::*;
pub use mesh::*;
pub use shader::*;
pub use texture::*;

use std::collections::HashMap;

// RendererHandle defines the handle to the renderer.
#[derive(Clone)]
pub struct RendererHandle {
    builtin_shaders: HashMap<BuiltinShader, ShaderHandle>,
    queue: RenderQueueWriter,
}

impl RendererHandle {
    pub(crate) fn new(queue: RenderQueueWriter) -> Self {
        let mut handle = Self {
            builtin_shaders: HashMap::new(),
            queue,
        };

        for (shader, def) in builtin_shader_definitions() {
            let shader_handle = handle.create_shader(def);
            handle.builtin_shaders.insert(shader, shader_handle);
        }

        handle
    }

    pub fn builtin_shader(&self, shader: BuiltinShader) -> Option<&ShaderHandle> {
        self.builtin_shaders.get(&shader)
    }

    pub fn create_material(&self, definition: MaterialDefinition) -> MaterialHandle {
        let handle = MaterialHandle::new();
        self.queue.push(RenderCommand::CreateMaterial(handle, definition));
        handle
    }
    
    pub fn create_mesh(&self, definition: MeshDefinition) -> MeshHandle {
        let handle = MeshHandle::new();
        self.queue.push(RenderCommand::CreateMesh(handle, definition));
        handle
    }

    pub fn create_shader(&self, definition: ShaderDefinition) -> ShaderHandle {
        let handle = ShaderHandle::new();
        self.queue.push(RenderCommand::CreateShader(handle, definition));
        handle
    }

    pub fn create_texture(&self, definition: TextureDefinition) -> TextureHandle {
        let handle = TextureHandle::new();
        self.queue.push(RenderCommand::CreateTexture(handle, definition));
        handle
    }

    pub fn draw(&self, phase: RenderPhase, cmd: DrawCommand) {
        self.queue.push(RenderCommand::Draw(phase, cmd));
    }

    pub fn render(&self, renderable: Box<dyn Renderable + Send>) {
        self.queue.push(RenderCommand::Render(renderable));
    }
}
