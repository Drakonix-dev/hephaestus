pub use builtin::BuiltinShader;
pub use camera::{OrthographicProjection, PerspectiveProjection, Projection};
pub use commands::{DrawCommand, DrawMesh, Renderable, Transform};
pub use graph::{RenderDomain, RenderGraph, RenderPhase, SubPhase};
pub use material::{MaterialDefinition, MaterialHandle, MaterialParams, MaterialUniforms};
pub use mesh::{MeshDefinition, MeshHandle, Vertex};
pub use shader::{
    BindGroupLayout, BindingDesc, BindingType, ShaderDefinition, ShaderHandle, ShaderStage,
};
pub use texture::{TextureDefinition, TextureDimension, TextureHandle};

pub(crate) use commands::{
    RenderCommand, RenderQueueReader, RenderQueueWriter, render_queue_channel,
};

mod builtin;
mod camera;
mod commands;
mod graph;
mod material;
mod mesh;
mod shader;
mod texture;

use builtin::builtin_shader_definitions;
use std::collections::HashMap;

use crate::{math::Mat4, renderer::RenderError};

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    width: u32,
    height: u32,
}

impl Viewport {
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.max(1),
            height: height.max(1),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
    Vsync,
    Immediate,
    Mailbox,
}

// RendererHandle defines the handle to the renderer.
pub struct RendererHandle {
    builtin_shaders: HashMap<BuiltinShader, ShaderHandle>,
    queue: RenderQueueWriter,
    viewport: Viewport,
}

impl RendererHandle {
    pub(crate) fn new(queue: RenderQueueWriter, viewport: Viewport) -> Self {
        let mut handle = Self {
            builtin_shaders: HashMap::new(),
            queue,
            viewport,
        };

        for (shader, def) in builtin_shader_definitions() {
            let shader_handle = handle.create_shader(def);
            handle.builtin_shaders.insert(shader, shader_handle);
        }

        handle
    }

    pub fn builtin_shader(&self, shader: BuiltinShader) -> Result<&ShaderHandle, RenderError> {
        self.builtin_shaders
            .get(&shader)
            .ok_or_else(|| RenderError::AssetNotFound {
                name: shader.to_string(),
            })
    }

    pub fn create_material(&self, definition: MaterialDefinition) -> MaterialHandle {
        let handle = MaterialHandle::new();
        self.queue
            .push(RenderCommand::CreateMaterial(handle, definition));
        handle
    }

    pub fn create_mesh(&self, definition: MeshDefinition) -> MeshHandle {
        let handle = MeshHandle::new();
        self.queue
            .push(RenderCommand::CreateMesh(handle, definition));
        handle
    }

    pub fn create_shader(&self, definition: ShaderDefinition) -> ShaderHandle {
        let handle = ShaderHandle::new();
        self.queue
            .push(RenderCommand::CreateShader(handle, definition));
        handle
    }

    pub fn create_texture(&self, definition: TextureDefinition) -> TextureHandle {
        let handle = TextureHandle::new();
        self.queue
            .push(RenderCommand::CreateTexture(handle, definition));
        handle
    }

    pub fn draw(&self, phase: RenderPhase, cmd: DrawCommand) {
        self.queue.push(RenderCommand::Draw(phase, cmd));
    }

    pub fn render(&self, renderable: Box<dyn Renderable + Send>) {
        self.queue.push(RenderCommand::Render(renderable));
    }

    pub fn set_camera(&self, view_proj: Mat4) {
        self.queue.push(RenderCommand::SetCamera(view_proj));
    }

    pub(crate) fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }

    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
}
