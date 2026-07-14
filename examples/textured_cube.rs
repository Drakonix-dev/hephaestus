use std::{path::PathBuf, time::Duration};

use hephaestus::{
    ApplicationContext, ApplicationInstance,
    config::EngineConfig,
    math::{Interpolated, Quat, Transform3D, Vec3},
    renderer::{
        BuiltinShader, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MaterialParams,
        MaterialUniforms, MeshDefinition, MeshHandle, PerspectiveProjection, Projection,
        RenderDomain, RenderGraph, RenderPhase, SubPhase, TextureDefinition, TextureDimension,
        Transform,
    },
};

const CUBE_PHASE: RenderPhase = RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Opaque));
const ROTATION_SPEED: f32 = 1.0;

struct TexturedCubeExample {
    angle: f32,
    camera: Transform3D,
    cube: Interpolated<Transform3D>,
    material: MaterialHandle,
    mesh: MeshHandle,
    projection: Projection,
}

impl TexturedCubeExample {
    fn new(ctx: &ApplicationContext) -> Self {
        let shader = *ctx
            .renderer
            .builtin_shader(BuiltinShader::UnlitTextured)
            .expect("unlit textured shader should be registered");

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

        let mesh = ctx.renderer.create_mesh(MeshDefinition::cube());

        let camera = Transform3D {
            position: Vec3::new(0.0, 0.0, 2.0),
            ..Transform3D::default()
        };

        let projection = Projection::Perspective(PerspectiveProjection {
            far: 100.0,
            fovy: 60.0_f32.to_radians(),
            near: 0.1,
        });

        Self {
            angle: 0.0,
            camera,
            cube: Interpolated::new(Transform3D::default()),
            material,
            mesh,
            projection,
        }
    }
}

impl ApplicationInstance for TexturedCubeExample {
    fn render(&mut self, phase: &RenderPhase, alpha: f32) -> Option<Vec<DrawCommand>> {
        if *phase != CUBE_PHASE {
            return None;
        }

        Some(vec![DrawCommand::Mesh(DrawMesh {
            mesh: self.mesh,
            material: self.material,
            transform: Transform::Transform3D(self.cube.sample(alpha)),
        })])
    }

    fn update(&mut self, ctx: &ApplicationContext, dt: Duration) {
        self.angle += ROTATION_SPEED * dt.as_secs_f32();
        let axis = Vec3::new(1.0, 1.0, 1.0).normalize();
        let mut cube = self.cube.current();
        cube.rotation = Quat::from_axis_angle(axis, self.angle);
        self.cube.set(cube);

        let view_proj = self.projection.project(ctx.viewport) * self.camera.view_matrix();
        ctx.renderer.set_camera(view_proj);
    }
}

fn main() {
    hephaestus::run_instance(
        TexturedCubeExample::new,
        EngineConfig::default(),
        RenderGraph::default(),
    )
    .expect("failed to run application");
}
