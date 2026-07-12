mod api;

pub(crate) mod backend;

pub use api::*;

use std::collections::HashMap;

use crate::{diagnostics::diag, math::Mat4, platform::core::PlatformError};

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
    fn set_fullscreen(&mut self, enabled: bool);
    fn set_present_mode(&mut self, mode: PresentMode);
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
        let span = tracing::debug_span!(
            target: diag::RENDER,
            "render",
            phases = self.phases.len(),
            draws = tracing::field::Empty,
        );
        let _enter = span.enter();

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

        span.record(diag::FIELD_DRAWS, total_draws);

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

    pub(crate) fn set_fullscreen(&mut self, enabled: bool) {
        self.backend.set_fullscreen(enabled)
    }

    pub(crate) fn set_present_mode(&mut self, mode: PresentMode) {
        self.backend.set_present_mode(mode)
    }

    pub(crate) fn shutdown(&mut self) {
        self.backend.shutdown();
    }

    // ------------------------------------------------------------------------

    fn process_staging_uploads(
        &mut self,
        staged_commands: &mut Vec<RenderCommand>,
    ) -> Result<(), PlatformError> {
        if staged_commands.is_empty() {
            return Ok(());
        }

        let _span = tracing::debug_span!(
            target: diag::UPLOAD,
            "process_staging_uploads",
            commands = staged_commands.len(),
        )
        .entered();

        for cmd in staged_commands.drain(..) {
            match cmd {
                RenderCommand::CreateMaterial(handle, def) => {
                    tracing::debug!(target: diag::ASSET, kind = "material", ?handle, "create");
                    self.backend.create_material(handle, def)?;
                }
                RenderCommand::CreateMesh(handle, def) => {
                    tracing::debug!(target: diag::ASSET, kind = "mesh", ?handle, "create");
                    self.backend.create_mesh(handle, def);
                }
                RenderCommand::CreateShader(handle, def) => {
                    tracing::debug!(target: diag::ASSET, kind = "shader", ?handle, "create");
                    self.backend.create_shader(handle, def)?;
                }
                RenderCommand::CreateTexture(handle, def) => {
                    tracing::debug!(target: diag::ASSET, kind = "texture", ?handle, "create");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::{capture, diag},
        math::Transform3D,
        renderer::{
            DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MeshDefinition, MeshHandle,
            RenderDomain, RenderGraph, RenderPhase, RenderQueue, ShaderDefinition, ShaderHandle,
            TextureDefinition, TextureHandle, Transform,
        },
    };

    #[derive(Default)]
    struct MockBackend;

    struct MockFrame;

    impl<'a> RenderFrame<'a> for MockFrame {
        fn execute_commands(
            &mut self,
            _phase: &RenderPhase,
            _cmds: &[DrawCommand],
        ) -> Result<(), PlatformError> {
            Ok(())
        }

        fn present_frame(self) {}
    }

    impl RendererBackend for MockBackend {
        type Frame<'a> = MockFrame;

        fn begin_frame(&mut self) -> Result<Self::Frame<'_>, PlatformError> {
            Ok(MockFrame)
        }

        fn create_material(
            &mut self,
            _handle: MaterialHandle,
            _definition: MaterialDefinition,
        ) -> Result<(), PlatformError> {
            Ok(())
        }

        fn create_mesh(&mut self, _handle: MeshHandle, _definition: MeshDefinition) {}

        fn create_shader(
            &mut self,
            _handle: ShaderHandle,
            _definition: ShaderDefinition,
        ) -> Result<(), PlatformError> {
            Ok(())
        }

        fn create_texture(
            &mut self,
            _handle: TextureHandle,
            _definition: TextureDefinition,
        ) -> Result<(), PlatformError> {
            Ok(())
        }

        fn reserve_draw_capacity(&mut self, _count: u64) {}

        fn resize(&mut self, _width: u32, _height: u32) {}

        fn set_camera(&mut self, _view_proj: Mat4) {}

        fn set_fullscreen(&mut self, _enabled: bool) {}

        fn set_present_mode(&mut self, _mode: PresentMode) {}

        fn shutdown(&mut self) {}
    }

    fn mesh_draw() -> DrawCommand {
        DrawCommand::Mesh(DrawMesh {
            mesh: MeshHandle::new(),
            material: MaterialHandle::new(),
            transform: Transform::Transform3D(Transform3D::default()),
        })
    }

    fn renderer_with_phase(
        phase: RenderPhase,
    ) -> (Renderer<MockBackend>, super::RenderQueueWriter) {
        let (writer, reader) = RenderQueue::new();
        let graph = RenderGraph::new().add_phase(phase);
        let renderer = Renderer::new(MockBackend, &graph, reader).unwrap();
        (renderer, writer)
    }

    #[test]
    fn render_span_records_total_draw_count() {
        let phase = RenderPhase::new(RenderDomain::World3D, None);
        let (mut renderer, _writer) = renderer_with_phase(phase);

        let capture = capture();
        renderer
            .render(|candidate| (*candidate == phase).then(|| vec![mesh_draw(), mesh_draw()]))
            .unwrap();

        let render = capture
            .find(|record| record.target == diag::RENDER && record.name == "render")
            .expect("render span should be captured");
        assert_eq!(render.field_u64(diag::FIELD_DRAWS), Some(2));
        assert_eq!(render.field_u64(diag::FIELD_PHASES), Some(1));
    }

    #[test]
    fn staging_uploads_emit_asset_create_events() {
        let phase = RenderPhase::new(RenderDomain::World3D, None);
        let (mut renderer, writer) = renderer_with_phase(phase);
        writer.push(RenderCommand::CreateMesh(
            MeshHandle::new(),
            MeshDefinition {
                vertices: Vec::new(),
                indices: Vec::new(),
            },
        ));

        let capture = capture();
        renderer.render(|_| None).unwrap();

        let created = capture
            .find(|record| record.target == diag::ASSET && record.field_str("kind") == Some("mesh"))
            .expect("asset create event should be captured");
        assert_eq!(created.kind, crate::diagnostics::RecordKind::Event);
    }
}
