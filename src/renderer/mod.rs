mod api;

pub(crate) mod backend;

pub use api::*;

use std::{collections::HashMap, sync::Arc};

pub(crate) trait RendererBackend {
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition);
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition);
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition);
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition);
    fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]);
    fn present_frame(&mut self);
    fn resize(&mut self, width: u32, height: u32);
}

pub(crate) struct Renderer<'a> {
    backend: &'a mut dyn RendererBackend,
    phases: Vec<RenderPhase>,
    queue: Arc<RenderCommandQueue>,
    staged_draws: HashMap<RenderPhase, Vec<DrawCommand>>,
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(
        backend: &'a mut dyn RendererBackend,
        graph: &RenderGraph,
        queue: Arc<RenderCommandQueue>,
    ) -> Self {
        let phases = graph.linearize()
            .expect("Invalid RenderGraph");

        Self {
            backend,
            phases,
            queue,
            staged_draws: HashMap::new(),
        }
    }

    pub(crate) fn redraw(&mut self) {
        let mut cmds = self.queue.swap();
        
        self.process_staging_uploads(&mut cmds);
        self.render();
    }

    // ------------------------------------------------------------------------

    fn process_staging_uploads(&mut self, staged_commands: &mut Vec<RenderCommand>) {
        for cmd in staged_commands.drain(..) {
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
                RenderCommand::Draw(phase, draw) => {
                    self.staged_draws.entry(phase).or_default().push(draw);
                },
                RenderCommand::Render(renderable) => {
                    for &phase in self.phases.iter() {
                        let cmds = self.staged_draws.entry(phase).or_default();
                        cmds.append(&mut renderable.draw(phase));
                    }
                }, 
            }
        }
    }

    fn render(&mut self) {
        for phase in &self.phases {
            if let Some(draws) = self.staged_draws.get_mut(phase) {
                self.backend.execute_commands(*phase, draws);
                draws.clear();
            }
        }

        self.backend.present_frame();
    }
}
