use crate::math::{Mat4, view};

pub struct Camera;

pub enum Projection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}

impl Projection {
    pub fn project(&self, aspect_ratio: f32) -> Mat4 {
        match self {
            Projection::Perspective(p) => view::perspective(p.fovy, aspect_ratio, p.near, p.far),
            Projection::Orthographic(o) => {
                let half_height = o.height / 2.0;
                let half_width = half_height * aspect_ratio;
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
