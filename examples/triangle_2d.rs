use std::{path::PathBuf, time::Duration};

use hephaestus::{
    ApplicationContext, ApplicationInstance,
    config::EngineConfig,
    math::{Transform3D, Vec3},
    renderer::{
        BuiltinShader, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MaterialParams,
        MaterialUniforms, MeshDefinition, MeshHandle, OrthographicProjection, Projection,
        RenderDomain, RenderGraph, RenderPhase, SubPhase, TextureDefinition, TextureDimension,
        Transform,
    },
};

const TRIANGLE_PHASE: RenderPhase = RenderPhase::new(RenderDomain::World2D, Some(SubPhase::Opaque));

struct TriangleExample {
    camera: Transform3D,
    material: MaterialHandle,
    mesh: MeshHandle,
    projection: Projection,
}

impl TriangleExample {
    fn new(ctx: &ApplicationContext) -> Self {
        let shader = *ctx
            .renderer
            .builtin_shader(BuiltinShader::UnlitTextured)
            .expect("simple color shader should be registered");

        let texture = ctx.renderer.create_texture(TextureDefinition {
            depth: None,
            dimension: TextureDimension::D2,
            source: PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/textures/checker.png"
            )),
        });

        let material = ctx.renderer.create_material(MaterialDefinition {
            shader,
            params: MaterialParams {
                textures: vec![texture],
                uniforms: MaterialUniforms {
                    base_color: [1.0, 1.0, 1.0, 1.0],
                },
            },
        });

        let mesh = ctx.renderer.create_mesh(MeshDefinition::triangle());

        let camera = Transform3D {
            position: Vec3::new(0.0, 0.0, 2.0),
            ..Transform3D::default()
        };

        let projection = Projection::Orthographic(OrthographicProjection {
            far: 100.0,
            height: 2.0,
            near: 0.1,
        });

        Self {
            camera,
            material,
            mesh,
            projection,
        }
    }
}

impl ApplicationInstance for TriangleExample {
    fn render(&mut self, phase: &RenderPhase, _alpha: f32) -> Option<Vec<DrawCommand>> {
        if *phase != TRIANGLE_PHASE {
            return None;
        }

        Some(vec![DrawCommand::Mesh(DrawMesh {
            mesh: self.mesh,
            material: self.material,
            transform: Transform::Transform3D(Transform3D::default()),
        })])
    }

    fn update(&mut self, ctx: &ApplicationContext, _dt: Duration) {
        let view_proj =
            self.projection.project(ctx.renderer.viewport()) * self.camera.view_matrix();
        ctx.renderer.set_camera(view_proj);
    }
}

fn main() {
    hephaestus::run_instance(
        TriangleExample::new,
        EngineConfig::default(),
        RenderGraph::default(),
    )
    .expect("failed to run application");
}
