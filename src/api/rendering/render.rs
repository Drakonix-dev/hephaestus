use std::{collections::HashMap, mem::take};

use crate::{builtin::BuiltinShader, core::ecs::World, rendering::{DrawMesh, MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle, ShaderDefinition, ShaderHandle, TextureDefinition, TextureHandle}};

// RenderPhase defines a phase of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderPhase {
    pub domain: RenderDomain,
    pub subphase: Option<SubPhase>,
}

impl RenderPhase {
    pub fn new(domain: RenderDomain, subphase: Option<SubPhase>) -> Self {
        Self { domain, subphase }
    }
}

// RenderDomain defines the different domains of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderDomain {
    UI,
    World2D,
    World3D,
    Other(u32),
}

// SubPhase defines the different sub-phases of rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubPhase {
    Opaque,
    Transparent,
    Other(u32),
}

// DrawCommand defines a command that can be translated to GPU calls.
pub enum DrawCommand {
    Mesh(DrawMesh),
}

// Renderable defines something that can be rendered.
pub trait Renderable {
    fn draw(&self, world: &World, phase: RenderPhase) -> Vec<DrawCommand>;
}

// RenderContext defines the view for systems and renderables.
pub struct RenderContext<'a> {
    pub(crate) phases: &'a mut HashMap<RenderPhase, Vec<DrawCommand>>,
}

impl<'a> RenderContext<'a> {
    pub fn draw(&mut self, phase: RenderPhase, cmd: DrawCommand) {
        self.phases.entry(phase).or_default().push(cmd);
    }
}
 
// RendererBackend defines how a backend for the renderer should behave.
pub(crate) trait RendererBackend {
    fn execute_commands(&mut self, phase: RenderPhase, cmds: &[DrawCommand]);
    fn present(&mut self);
    fn resize(&mut self, width: u32, height: u32);
    
    fn builtin_shader(&self, shader: &BuiltinShader) -> ShaderHandle;
    
    fn create_material(&mut self, def: &MaterialDefinition) -> MaterialHandle;
    fn create_mesh(&mut self, def: &MeshDefinition) -> MeshHandle;
    fn create_shader(&mut self, def: &ShaderDefinition) -> ShaderHandle;
    fn create_texture(&mut self, def: &TextureDefinition) -> TextureHandle;
}

pub struct RenderSubmission {
    pub renderables: Vec<Box<dyn Renderable>>,
    pub manual_draws: Option<Box<dyn FnOnce(&mut RenderContext) + Send>>,
}

// Renderer defines the type that performs all the rendering.
pub struct Renderer {
    backend: Box<dyn RendererBackend>,
    next_phase_order: Vec<RenderPhase>,
    phase_order: Vec<RenderPhase>,
    phases: HashMap<RenderPhase, Vec<DrawCommand>>,
    queued_submissions: Vec<RenderSubmission>,
}

impl Renderer {
    pub(crate) fn new(backend: Box<dyn RendererBackend>) -> Self {
        let phase_order = vec![
            RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Opaque)),
            RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Transparent)),
            RenderPhase::new(RenderDomain::World2D, Some(SubPhase::Opaque)),
            RenderPhase::new(RenderDomain::World2D, Some(SubPhase::Transparent)),
            RenderPhase::new(RenderDomain::UI, None),
        ];
        
        Self {
            backend,
            next_phase_order: Vec::new(),
            phase_order,
            phases: HashMap::new(),
            queued_submissions: Vec::new(),
        }
    }

    fn begin_frame(&mut self) {
        for v in self.phases.values_mut() {
            v.clear();
        }
    }

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

    fn finish_frame(&mut self) {
        self.backend.present();
    }
    
    pub fn queue_render(
        &mut self,
        renderables: Vec<Box<dyn Renderable>>,
        manual_draws: Option<Box<dyn FnOnce(&mut RenderContext) + Send>>,
    ) {
        self.queued_submissions.push(RenderSubmission {
            renderables,
            manual_draws,
        })
    }

    pub(crate) fn render(&mut self, world: &World) {
        if !self.next_phase_order.is_empty() {
            self.phase_order = take(&mut self.next_phase_order);
        }
        
        self.begin_frame();

        let submissions = take(&mut self.queued_submissions);
        for submission in submissions {
            if let Some(draw_fn) = submission.manual_draws {
                draw_fn(&mut RenderContext { phases: &mut self.phases });
            }

            for r in &submission.renderables {
                for &phase in &self.phase_order {
                    let cmds = self.phases.entry(phase).or_default();
                    cmds.append(&mut r.draw(world, phase));
                }
            }
        }
        
        for &phase in self.phase_order.iter() {
            let cmds = self.phases.entry(phase).or_default();

            // TODO: draw entities in world.

            self.backend.execute_commands(phase, cmds);
        } 

        self.finish_frame();
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.backend.resize(width, height)
    }

    pub fn set_phrase_order(&mut self, phases: Vec<RenderPhase>) {
        self.next_phase_order = phases;
    }
}
