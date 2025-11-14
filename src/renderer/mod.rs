mod api;

pub(crate) mod backend;

pub use api::*;

use std::{collections::HashMap, marker::PhantomData};

use crate::platform::core::PlatformError;

pub(crate) trait RenderFrame {
    fn execute_commands(&mut self, phase: &RenderPhase, cmds: &[DrawCommand]) -> Result<(), PlatformError>;
    fn present_frame(self);
}

pub(crate) trait RendererBackend<'a, F: RenderFrame + 'a> {
    fn begin_frame(&'a mut self) -> Result<F, PlatformError>;
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition) -> Result<(), PlatformError>;
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition);
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition) -> Result<(), PlatformError>;
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition) -> Result<(), PlatformError>;
    fn resize(&mut self, width: u32, height: u32);
    fn shutdown(&mut self);
}

pub(crate) struct Renderer<'a, B: RendererBackend<'a, F>, F: RenderFrame + 'a> {
    backend: B,
    phases: Vec<RenderPhase>,
    queue: RenderQueueReader,
    staged_draws: HashMap<RenderPhase, Vec<DrawCommand>>,
    
    _marker: PhantomData<F>,
}

impl<B: RendererBackend<F>, F: RenderFrame> Renderer<B, F> {
    pub(crate) fn new(
        backend: B,
        graph: &RenderGraph,
        queue: RenderQueueReader,
    ) -> Result<Self, PlatformError> {
        let phases = graph.linearize()
            .map_err(|err| PlatformError::BadRenderGraph(err.to_string()))?;

        Ok(Self {
            backend,
            phases,
            queue,
            staged_draws: HashMap::new(),
            _marker: PhantomData,
        })
    }

    pub(crate) fn render<T>(&mut self, phased_draws: T) -> Result<(), PlatformError>
        where T: FnMut(&RenderPhase) -> Option<Vec<DrawCommand>>
    {
        let mut cmds = self.queue.drain();
        let mut phased_draws = phased_draws;
        
        self.process_staging_uploads(&mut cmds)?;
        
        let mut frame = self.backend.begin_frame()?;
        
        for phase in &self.phases {
            let mut extra = phased_draws(phase)
                .unwrap_or_default();
            
            if let Some(draws) = self.staged_draws.get_mut(phase) {
                extra.append(draws);
            }
            
            frame.execute_commands(phase, &extra)?;
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
                },
                RenderCommand::CreateMesh(handle, def) => {
                    self.backend.create_mesh(handle, def);
                },
                RenderCommand::CreateShader(handle, def) => {
                    self.backend.create_shader(handle, def)?;
                },
                RenderCommand::CreateTexture(handle, def) => {
                    self.backend.create_texture(handle, def)?;
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

        Ok(())
    }
}
