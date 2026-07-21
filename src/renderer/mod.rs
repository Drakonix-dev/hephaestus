mod camera;
mod commands;
mod error;
mod graph;
mod material;
mod mesh;
mod shader;
mod texture;
pub(crate) mod wgpu;

pub use camera::{OrthographicProjection, PerspectiveProjection, Projection, Viewport};
pub use commands::{DrawCommand, DrawMesh, Renderable, Transform};
pub use error::{FrameError, RenderError};
pub use graph::{RenderDomain, RenderGraph, RenderPhase, SubPhase};
pub use material::{Material, MaterialDefinition, MaterialUniforms};
pub use mesh::{Mesh, MeshDefinition, Vertex};
pub use shader::{
    BindGroupLayout, BindingDesc, BindingType, Shader, ShaderDefinition, ShaderStage,
};
pub use texture::{Texture, TextureDefinition, TextureDimension};

// use crate::{diagnostics::diag, math::Mat4};
// use std::collections::HashMap;
//
// pub(crate) trait RenderFrame<'a> {
//     fn execute_commands(
//         &mut self,
//         phase: &RenderPhase,
//         cmds: &[DrawCommand],
//     ) -> Result<(), RenderError>;
//     fn present_frame(self);
// }
//
// pub(crate) trait RendererBackend {
//     type Frame<'a>: RenderFrame<'a>
//     where
//         Self: 'a;
//
//     fn begin_frame<'a>(&'a mut self) -> Result<Self::Frame<'a>, RenderError>;
//     fn create_material(
//         &mut self,
//         handle: MaterialHandle,
//         definition: MaterialDefinition,
//     ) -> Result<(), RenderError>;
//     fn create_mesh(&mut self, handle: MeshHandle, definition: MeshDefinition);
//     fn create_shader(
//         &mut self,
//         handle: ShaderHandle,
//         definition: ShaderDefinition,
//     ) -> Result<(), RenderError>;
//     fn create_texture(
//         &mut self,
//         handle: TextureHandle,
//         definition: TextureDefinition,
//     ) -> Result<(), RenderError>;
//     fn reconfigure(&mut self);
//     fn reserve_draw_capacity(&mut self, count: u64);
//     fn resize(&mut self, width: u32, height: u32);
//     fn set_camera(&mut self, view_proj: Mat4);
//     fn set_fullscreen(&mut self, enabled: bool);
//     fn set_present_mode(&mut self, mode: PresentMode);
//     fn shutdown(&mut self);
//     fn viewport(&self) -> Viewport;
// }
//
// pub(crate) struct Renderer<B: RendererBackend> {
//     backend: B,
//     phases: Vec<RenderPhase>,
//     queue: RenderQueueReader,
//     staged_draws: HashMap<RenderPhase, Vec<DrawCommand>>,
// }
//
// pub(crate) struct PreparedFrame {
//     draws_by_phase: Vec<(RenderPhase, Vec<DrawCommand>)>,
// }
//
// impl<B: RendererBackend> Renderer<B> {
//     pub(crate) fn new(
//         backend: B,
//         graph: &RenderGraph,
//         queue: RenderQueueReader,
//     ) -> Result<Self, RenderError> {
//         let phases = graph
//             .linearize()
//             .map_err(|err| RenderError::BadRenderGraph {
//                 detail: err.to_string(),
//             })?;
//
//         Ok(Self {
//             backend,
//             phases,
//             queue,
//             staged_draws: HashMap::new(),
//         })
//     }
//
//     pub(crate) fn prepare<T>(&mut self, mut phased_draws: T) -> Result<PreparedFrame, RenderError>
//     where
//         T: FnMut(&RenderPhase) -> Option<Vec<DrawCommand>>,
//     {
//         let span = tracing::debug_span!(
//             target: diag::RENDER,
//             "render",
//             phases = self.phases.len(),
//             draws = tracing::field::Empty,
//         );
//         let _enter = span.enter();
//
//         let mut cmds = self.queue.drain();
//         self.process_staging_uploads(&mut cmds)?;
//
//         let mut draws_by_phase: Vec<(RenderPhase, Vec<DrawCommand>)> =
//             Vec::with_capacity(self.phases.len());
//         let mut total_draws: u64 = 0;
//
//         for phase in self.phases.iter().copied() {
//             let mut extra = phased_draws(&phase).unwrap_or_default();
//
//             if let Some(draws) = self.staged_draws.get_mut(&phase) {
//                 extra.append(draws);
//             }
//
//             total_draws += extra.len() as u64;
//             draws_by_phase.push((phase, extra));
//         }
//
//         span.record(diag::FIELD_DRAWS, total_draws);
//
//         self.backend.reserve_draw_capacity(total_draws);
//
//         Ok(PreparedFrame { draws_by_phase })
//     }
//
//     pub(crate) fn reconfigure(&mut self) {
//         self.backend.reconfigure();
//     }
//
//     pub(crate) fn resize(&mut self, width: u32, height: u32) {
//         self.backend.resize(width, height)
//     }
//
//     pub(crate) fn set_fullscreen(&mut self, enabled: bool) {
//         self.backend.set_fullscreen(enabled)
//     }
//
//     pub(crate) fn set_present_mode(&mut self, mode: PresentMode) {
//         self.backend.set_present_mode(mode)
//     }
//
//     pub(crate) fn shutdown(&mut self) {
//         self.backend.shutdown();
//     }
//
//     pub(crate) fn submit(&mut self, prepared: &PreparedFrame) -> Result<(), RenderError> {
//         let mut frame = self.backend.begin_frame()?;
//
//         for (phase, cmds) in &prepared.draws_by_phase {
//             frame.execute_commands(phase, cmds)?;
//         }
//
//         frame.present_frame();
//
//         Ok(())
//     }
//
//     pub(crate) fn viewport(&self) -> Viewport {
//         self.backend.viewport()
//     }
//
//     // ------------------------------------------------------------------------
//
//     fn process_staging_uploads(
//         &mut self,
//         staged_commands: &mut Vec<RenderCommand>,
//     ) -> Result<(), RenderError> {
//         if staged_commands.is_empty() {
//             return Ok(());
//         }
//
//         let _span = tracing::debug_span!(
//             target: diag::UPLOAD,
//             "process_staging_uploads",
//             commands = staged_commands.len(),
//         )
//         .entered();
//
//         for cmd in staged_commands.drain(..) {
//             match cmd {
//                 RenderCommand::CreateMaterial(handle, def) => {
//                     tracing::debug!(target: diag::ASSET, kind = "material", ?handle, "create");
//                     self.backend.create_material(handle, def)?;
//                 }
//                 RenderCommand::CreateMesh(handle, def) => {
//                     tracing::debug!(target: diag::ASSET, kind = "mesh", ?handle, "create");
//                     self.backend.create_mesh(handle, def);
//                 }
//                 RenderCommand::CreateShader(handle, def) => {
//                     tracing::debug!(target: diag::ASSET, kind = "shader", ?handle, "create");
//                     self.backend.create_shader(handle, def)?;
//                 }
//                 RenderCommand::CreateTexture(handle, def) => {
//                     tracing::debug!(target: diag::ASSET, kind = "texture", ?handle, "create");
//                     self.backend.create_texture(handle, def)?;
//                 }
//                 RenderCommand::Draw(phase, draw) => {
//                     self.staged_draws.entry(phase).or_default().push(draw);
//                 }
//                 RenderCommand::Render(renderable) => {
//                     for &phase in self.phases.iter() {
//                         let cmds = self.staged_draws.entry(phase).or_default();
//                         cmds.append(&mut renderable.draw(phase));
//                     }
//                 }
//                 RenderCommand::SetCamera(view_proj) => {
//                     self.backend.set_camera(view_proj);
//                 }
//             }
//         }
//
//         Ok(())
//     }
// }
