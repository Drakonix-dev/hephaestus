use crate::math::{Quat, Vec2, Vec3};

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self(glam::Vec2::new(x, y))
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Self(glam::Vec2::ZERO)
    }
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self(glam::Vec3::new(x, y, z))
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self(glam::Vec3::ZERO)
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self(glam::Quat::IDENTITY)
    }
}
