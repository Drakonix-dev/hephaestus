mod api;

pub(crate) mod backend;

pub use api::*;

use std::{collections::HashMap, mem::take, sync::mpsc::{channel, Receiver}, thread::{self, JoinHandle}};

pub(crate) trait RendererBackend {
    fn begin_frame(&mut self);
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition);
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition);
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition);
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition);
    fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]);
    fn init(&mut self);
    fn present_frame(&mut self);
}

pub(crate) struct RendererThread {
    backend: Box<dyn RendererBackend + Send>,
    phases: Vec<RenderPhase>,
    receiver: Receiver<RenderCommand>,
    staged_commands: Vec<RenderCommand>,
    staged_draws: HashMap<RenderPhase, Vec<DrawCommand>>,
}

impl RendererThread {
    pub(crate) fn spawn(backend: Box<dyn RendererBackend + Send>, graph: RenderGraph) -> (RendererHandle, JoinHandle<()>) {
        let (sender, receiver) = channel();
        let handle = RendererHandle::new(sender);
        let phases = graph.linearize()
            .expect("Invalid RenderGraph");

        let join = thread::spawn(move || {
           let mut renderer = RendererThread {
               backend,
               phases,
               receiver,
               staged_commands: Vec::new(),
               staged_draws: HashMap::new(),
           };
           renderer.run();
        });

        (handle, join)
    }

    fn collect_pending_commands(&mut self) {
        while let Ok(cmd) = self.receiver.try_recv() {
            match cmd {
                RenderCommand::Draw(phase, draw) => {
                    self.staged_draws.entry(phase).or_default().push(draw);
                },
                RenderCommand::Render(renderable) => {
                    for &phase in self.phases.iter() {
                        let cmds = self.staged_draws.entry(phase).or_default();
                        cmds.append(&mut renderable.draw(phase));
                    }
                },
                _ => {
                    self.staged_commands.push(cmd);
                },
            }
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
        let mut frame_draws = take(&mut self.staged_draws);
        self.backend.begin_frame();

        for &phase in self.phases.iter() {
            let cmds = frame_draws.entry(phase).or_default();
            self.backend.execute_commands(phase, &cmds);
        }

        self.backend.present_frame();
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
