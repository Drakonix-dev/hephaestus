mod api;

pub(crate) mod backend;

use std::{sync::mpsc::{channel, Receiver}, thread::{self, JoinHandle}};

pub use api::*;

pub(crate) trait RendererBackend {
    fn begin_frame(&mut self);
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition);
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition);
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition);
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition);
    fn init(&mut self);
    fn present_frame(&mut self);
}

pub(crate) struct RendererThread {
    backend: Box<dyn RendererBackend + Send>,
    receiver: Receiver<RenderCommand>,
    staged_commands: Vec<RenderCommand>,
}

impl RendererThread {
    pub(crate) fn spawn(backend: Box<dyn RendererBackend + Send>) -> (RendererHandle, JoinHandle<()>) {
        let (sender, receiver) = channel();
        let handle = RendererHandle::new(sender);

        let join = thread::spawn(move || {
           let mut renderer = RendererThread {
               backend,
               receiver,
               staged_commands: Vec::new(),
           };
           renderer.run();
        });

        (handle, join)
    }

    fn collect_pending_commands(&mut self) {
        while let Ok(cmd) = self.receiver.try_recv() {
            self.staged_commands.push(cmd);
        }
    }

    fn process_staging_uploads(&mut self) {
        for cmd in self.staged_commands.drain(..) {
            match cmd {
                RenderCommand::CreateMaterial(handle, def) => {
                    self.backend.create_material(handle, def);
                },
                RenderCommand::CreateMesh(handle, def) => {
                    self.backend.create_mesh(handle, def);
                },
                RenderCommand::CreateShader(handle, def) => {
                    self.backend.create_shader(handle, def);
                },
                RenderCommand::CreateTexture(handle, def) => {
                    self.backend.create_texture(handle, def);
                },
                _ => {},
            }
        }
    }

    fn render(&mut self) {
        
    }

    fn run(&mut self) {
        self.backend.init();
        
        loop {
            self.collect_pending_commands();
            self.process_staging_uploads();
            self.render();
        }
    }
}
