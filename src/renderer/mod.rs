mod api;

pub(crate) mod backend;

pub use api::*;

use std::{collections::HashMap, mem::swap, sync::{mpsc::channel, Arc, Mutex}, thread::{self, JoinHandle}};

pub(crate) trait RendererBackend {
    fn create_material(&mut self, handle: MaterialHandle, definition: MaterialDefinition);
    fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition);
    fn create_shader(&mut self, handle: ShaderHandle, definition: ShaderDefinition);
    fn create_texture(&mut self, handle: TextureHandle, definition: TextureDefinition);
    fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]);
    fn present_frame(&mut self);
    fn resize(&mut self, width: u32, height: u32);
}

pub(crate) struct RendererThread {
    backend: Box<dyn RendererBackend + Send>,
    phases: Vec<RenderPhase>,
    staged_draws: HashMap<RenderPhase, Vec<DrawCommand>>,
}

impl RendererThread {
    pub(crate) fn spawn<F>(graph: RenderGraph, create_backend: F) -> (RendererHandle, JoinHandle<()>)
        where F: FnOnce() -> Box<dyn RendererBackend + Send> + Send + 'static,
    {
        let (sender, receiver) = channel();
        let handle = RendererHandle::new(sender);
        
        let phases = graph.linearize()
            .expect("Invalid RenderGraph");

        let staged_consumer = Arc::new(Mutex::new(Vec::new()));
        let staged = staged_consumer.clone();

        thread::spawn(move || {
            while let Ok(cmd) = receiver.recv() {
                let mut staging = staged_consumer.lock().unwrap();
                
                match cmd {
                    RenderCommand::Quit => {
                        staging.push(cmd);
                        return;  
                    },
                    _ => {},
                }
                
                staging.push(cmd);
            }
        });

        let join = thread::spawn(move || {
            let backend = create_backend();
            
            let mut renderer = RendererThread {
                backend,
                phases,
                staged_draws: HashMap::new(),
            };
           
           renderer.run(staged.clone());
        });

        (handle, join)
    }

    fn process_staging_uploads(&mut self, staged_commands: &mut Vec<RenderCommand>) -> bool {
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
                RenderCommand::Quit => {
                    return true;
                },
            }
        }

        false
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

    fn run(&mut self, staged_commands: Arc<Mutex<Vec<RenderCommand>>>) {
        let mut staged = Vec::new();
        
        loop {
            let mut cmds = staged_commands.lock().unwrap();
            swap(&mut *cmds, &mut staged);
            
            if self.process_staging_uploads(&mut staged) {
                break;
            }
            
            self.render();

            staged.clear();
        }
    }
}
