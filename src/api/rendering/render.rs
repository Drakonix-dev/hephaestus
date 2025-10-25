use std::{collections::HashMap, mem::take, path::Path};

use crate::{core::ecs::World, rendering::{DrawMesh, MaterialDefinition, MaterialHandle, ShaderHandle, TextureDefinition, TextureHandle}};

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
    
    fn create_material(&mut self, def: &MaterialDefinition) -> MaterialHandle;
    fn create_shader(&mut self, path: &Path) -> ShaderHandle;
    fn create_texture(&mut self, def: &TextureDefinition) -> TextureHandle;
}

// Renderer defines the type that performs all the rendering.
pub struct Renderer {
    backend: Box<dyn RendererBackend>,
    next_phase_order: Vec<RenderPhase>,
    phase_order: Vec<RenderPhase>,
    phases: HashMap<RenderPhase, Vec<DrawCommand>>,
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
        }
    }

    fn begin_frame(&mut self) {
        for v in self.phases.values_mut() {
            v.clear();
        }
    }
    
    pub fn create_material(&mut self, def: &MaterialDefinition) -> MaterialHandle {
        self.backend.create_material(def)
    }

    pub fn create_shader(&mut self, path: &Path) -> ShaderHandle {
        self.backend.create_shader(path)
    }

    pub fn create_texture(&mut self, def: &TextureDefinition) -> TextureHandle {
        self.backend.create_texture(def)
    }

    fn finish_frame(&mut self) {
        self.backend.present();
    }

    pub fn render<F: FnOnce(&mut RenderContext)>(&mut self, world: &World, renderables: &[Box<dyn Renderable>], manual_draws: F) {
        if !self.next_phase_order.is_empty() {
            self.phase_order = take(&mut self.next_phase_order);
        }
        
        self.begin_frame();

        manual_draws(&mut RenderContext { phases: &mut self.phases });
        
        for &phase in self.phase_order.iter() {
            let cmds = self.phases.entry(phase).or_default();
            
            for r in renderables {
                cmds.append(&mut r.draw(world, phase));
            }

            // TODO: draw entities in world.

            self.backend.execute_commands(phase, cmds);
        } 

        self.finish_frame();
    }

    pub fn set_phrase_order(&mut self, phases: Vec<RenderPhase>) {
        self.next_phase_order = phases;
    }
}
