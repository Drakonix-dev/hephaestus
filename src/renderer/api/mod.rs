mod builtin;
mod commands;
mod graph;
mod material;
mod mesh;
mod shader;
mod texture;

use std::sync::mpsc::Sender;

pub use builtin::*;
pub use commands::*;
pub use graph::*;
pub use material::*;
pub use mesh::*;
pub use shader::*;
pub use texture::*;

// RendererHandle defines the handle to the renderer.
pub struct RendererHandle {
    next_material_id: MaterialHandle,
    next_mesh_id: MeshHandle,
    next_shader_id: ShaderHandle,
    next_texture_id: TextureHandle,
    sender: Sender<RenderCommand>,
}

impl RendererHandle {
    pub(crate) fn new(sender: Sender<RenderCommand>) -> Self {
        Self {
            next_material_id: MaterialHandle::new(),
            next_mesh_id: MeshHandle::new(),
            next_shader_id: ShaderHandle::new(),
            next_texture_id: TextureHandle::new(),
            sender,
        }
    }

    pub fn create_material(&mut self, definition: MaterialDefinition) -> MaterialHandle {
        let handle = self.next_material_id.next();
        
        self.sender.send(RenderCommand::CreateMaterial {
            handle,
            definition,
        });

        handle
    }
    
    pub fn create_mesh(&mut self, definition: MeshDefinition) -> MeshHandle {
        let handle = self.next_mesh_id.next();
        
        self.sender.send(RenderCommand::CreateMesh {
            handle,
            definition,
        });

        handle
    }

    pub fn create_shader(&mut self, definition: ShaderDefinition) -> ShaderHandle {
        let handle = self.next_shader_id.next();
        
        self.sender.send(RenderCommand::CreateShader {
            handle,
            definition,
        });

        handle
    }

    pub fn create_texture(&mut self, definition: TextureDefinition) -> TextureHandle {
        let handle = self.next_texture_id.next();
        
        self.sender.send(RenderCommand::CreateTexture {
            handle,
            definition,
        });

        handle
    }

    pub fn draw_frame(&self) {
        self.sender.send(RenderCommand::DrawFrame);
    }

    pub fn queue_draw(&self, phase: RenderPhase, cmd: DrawCommand) {
        self.sender.send(RenderCommand::QueueDraw {
            phase,
            cmd,
        });
    }

    pub fn register(&self, renderable: Box<dyn Renderable>) {
        self.sender.send(RenderCommand::Register(renderable));
    }
}
