use crate::math::{Mat4, view};

pub struct Camera;

pub enum Projection {
    Perspective(PerspectiveProjection),
}

impl Projection {
    pub fn project(&self, aspect_ratio: f32) -> Mat4 {
        match self {
            Projection::Perspective(p) => p.project(aspect_ratio),
        }
    }
}

pub struct PerspectiveProjection {
    pub far: f32,
    pub fovy: f32,
    pub near: f32,
}

impl PerspectiveProjection {
    fn project(&self, aspect_ratio: f32) -> Mat4 {
        view::perspective(self.fovy, aspect_ratio, self.near, self.far)
    }
}
