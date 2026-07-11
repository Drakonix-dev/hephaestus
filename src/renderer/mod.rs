mod api;

pub(crate) mod backend;

pub use api::*;

use std::collections::HashMap;

use crate::{math::Mat4, platform::core::PlatformError};

pub(crate) trait RenderFrame<'a> {
    fn execute_commands(
        &mut self,
        phase: &RenderPhase,
        cmds: &[DrawCommand],
    ) -> Result<(), PlatformError>;
    fn present_frame(self);
}

pub(crate) trait RendererBackend {
    type Frame<'a>: RenderFrame<'a>
    where
        Self: 'a;

    fn begin_frame<'a>(&'a mut self) -> Result<Self::Frame<'a>, PlatformError>;
    fn create_material(
        &mut self,
        handle: MaterialHandle,
        definition: MaterialDefinition,
    ) -> Result<(), PlatformError>;
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition);
    fn create_shader(
        &mut self,
        handle: ShaderHandle,
        definition: ShaderDefinition,
    ) -> Result<(), PlatformError>;
    fn create_texture(
        &mut self,
        handle: TextureHandle,
        definition: TextureDefinition,
    ) -> Result<(), PlatformError>;
    fn reserve_draw_capacity(&mut self, count: u64);
    fn resize(&mut self, width: u32, height: u32);
    fn set_camera(&mut self, view_proj: Mat4);
    fn shutdown(&mut self);
}

pub(crate) struct Renderer<B: RendererBackend> {
    backend: B,
    phases: Vec<RenderPhase>,
    queue: RenderQueueReader,
    staged_draws: HashMap<RenderPhase, Vec<DrawCommand>>,
}

impl<B: RendererBackend> Renderer<B> {
    pub(crate) fn new(
        backend: B,
        graph: &RenderGraph,
        queue: RenderQueueReader,
    ) -> Result<Self, PlatformError> {
        let phases = graph
            .linearize()
            .map_err(|err| PlatformError::BadRenderGraph(err.to_string()))?;

        Ok(Self {
            backend,
            phases,
            queue,
            staged_draws: HashMap::new(),
        })
    }

    pub(crate) fn render<T>(&mut self, phased_draws: T) -> Result<(), PlatformError>
    where
        T: FnMut(&RenderPhase) -> Option<Vec<DrawCommand>>,
    {
        let mut cmds = self.queue.drain();
        let mut phased_draws = phased_draws;

        self.process_staging_uploads(&mut cmds)?;

        let mut draws_by_phase: Vec<(RenderPhase, Vec<DrawCommand>)> =
            Vec::with_capacity(self.phases.len());
        let mut total_draws: u64 = 0;

        for phase in self.phases.iter().copied() {
            let mut extra = phased_draws(&phase).unwrap_or_default();

            if let Some(draws) = self.staged_draws.get_mut(&phase) {
                extra.append(draws);
            }

            total_draws += extra.len() as u64;
            draws_by_phase.push((phase, extra));
        }

        self.backend.reserve_draw_capacity(total_draws);

        let mut frame = self.backend.begin_frame()?;

        for (phase, cmds) in &draws_by_phase {
            frame.execute_commands(phase, cmds)?;
        }

        frame.present_frame();

        Ok(())
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.backend.resize(width, height)
    }

    pub(crate) fn shutdown(&mut self) {
        self.backend.shutdown();
    }

    // ------------------------------------------------------------------------

    fn process_staging_uploads(
        &mut self,
        staged_commands: &mut Vec<RenderCommand>,
    ) -> Result<(), PlatformError> {
        for cmd in staged_commands.drain(..) {
            match cmd {
                RenderCommand::CreateMaterial(handle, def) => {
                    self.backend.create_material(handle, def)?;
                }
                RenderCommand::CreateMesh(handle, def) => {
                    self.backend.create_mesh(handle, def);
                }
                RenderCommand::CreateShader(handle, def) => {
                    self.backend.create_shader(handle, def)?;
                }
                RenderCommand::CreateTexture(handle, def) => {
                    self.backend.create_texture(handle, def)?;
                }
                RenderCommand::Draw(phase, draw) => {
                    self.staged_draws.entry(phase).or_default().push(draw);
                }
                RenderCommand::Render(renderable) => {
                    for &phase in self.phases.iter() {
                        let cmds = self.staged_draws.entry(phase).or_default();
                        cmds.append(&mut renderable.draw(phase));
                    }
                }
                RenderCommand::SetCamera(view_proj) => {
                    self.backend.set_camera(view_proj);
                }
            }
        }

        Ok(())
    }
}
