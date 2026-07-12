use std::time::Duration;

use hephaestus::{
    ApplicationContext, ApplicationInstance,
    config::EngineConfig,
    math::{Quat, Transform3D, Vec3},
    renderer::{
        BuiltinShader, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle, MaterialParams,
        MaterialUniforms, MeshDefinition, MeshHandle, PerspectiveProjection, Projection,
        RenderDomain, RenderGraph, RenderPhase, SubPhase, Transform,
    },
};

const CUBE_PHASE: RenderPhase = RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Opaque));
const ROTATION_SPEED: f32 = 1.0;

struct CubeExample {
    angle: f32,
    camera: Transform3D,
    cube: Transform3D,
    material: MaterialHandle,
    mesh: MeshHandle,
    projection: Projection,
}

impl CubeExample {
    fn new(ctx: &ApplicationContext) -> Self {
        let shader = *ctx
            .renderer
            .builtin_shader(BuiltinShader::SimpleColor)
            .expect("simple color shader should be registered");

        let material = ctx.renderer.create_material(MaterialDefinition {
            shader,
            params: MaterialParams {
                textures: vec![],
                uniforms: MaterialUniforms {
                    base_color: [1.0, 0.2, 0.2, 1.0],
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
            cube: Transform3D::default(),
            material,
            mesh,
            projection,
        }
    }
}

impl ApplicationInstance for CubeExample {
    fn render(&mut self, phase: &RenderPhase) -> Option<Vec<DrawCommand>> {
        if *phase != CUBE_PHASE {
            return None;
        }

        Some(vec![DrawCommand::Mesh(DrawMesh {
            mesh: self.mesh,
            material: self.material,
            transform: Transform::Transform3D(self.cube),
        })])
    }

    fn update(&mut self, ctx: &ApplicationContext, dt: Duration) {
        self.angle += ROTATION_SPEED * dt.as_secs_f32();
        let axis = Vec3::new(1.0, 1.0, 1.0).normalize();
        self.cube.rotation = Quat::from_axis_angle(axis, self.angle);

        let view_proj =
            self.projection.project(ctx.renderer.viewport()) * self.camera.view_matrix();
        ctx.renderer.set_camera(view_proj);
    }
}

fn main() {
    hephaestus::run_instance(
        CubeExample::new,
        EngineConfig::default(),
        RenderGraph::default(),
    )
    .expect("failed to run application");
}
