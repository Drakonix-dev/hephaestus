use hephaestus::{
    ApplicationContext, ApplicationInstance,
    math::Transform3D,
    renderer::{
        BuiltinShader, DrawCommand, DrawMesh, MaterialDefinition, MaterialHandle,
        MaterialParams, MaterialUniforms, MeshDefinition, MeshHandle, RenderDomain, RenderGraph,
        RenderPhase, SubPhase, Transform,
    },
};

const TRIANGLE_PHASE: RenderPhase = RenderPhase::new(RenderDomain::World3D, Some(SubPhase::Opaque));

struct TriangleExample {
    mesh: MeshHandle,
    material: MaterialHandle,
}

impl TriangleExample {
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
                    roughness: 1.0,
                    metallic: 0.0,
                },
            },
        });

        let mesh = ctx.renderer.create_mesh(MeshDefinition::triangle());

        Self { mesh, material }
    }
}

impl ApplicationInstance for TriangleExample {
    fn render(&mut self, phase: &RenderPhase) -> Option<Vec<DrawCommand>> {
        if *phase != TRIANGLE_PHASE {
            return None;
        }

        Some(vec![DrawCommand::Mesh(DrawMesh {
            mesh: self.mesh,
            material: self.material,
            transform: Transform::Transform3D(Transform3D::default()),
        })])
    }
}

fn main() {
    hephaestus::run_instance(TriangleExample::new, RenderGraph::default())
        .expect("failed to run application");
}
