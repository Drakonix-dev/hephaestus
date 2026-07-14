use crate::{
    math::{Mat4, view},
    renderer::Viewport,
};

pub enum Projection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}

impl Projection {
    pub fn project(&self, viewport: &Viewport) -> Mat4 {
        match self {
            Projection::Perspective(p) => {
                view::perspective(p.fovy, viewport.aspect_ratio(), p.near, p.far)
            }
            Projection::Orthographic(o) => {
                let half_height = o.height / 2.0;
                let half_width = half_height * viewport.aspect_ratio();
                view::orthographic(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    o.near,
                    o.far,
                )
            }
        }
    }
}

pub struct PerspectiveProjection {
    pub far: f32,
    pub fovy: f32,
    pub near: f32,
}

pub struct OrthographicProjection {
    pub far: f32,
    pub height: f32,
    pub near: f32,
}
