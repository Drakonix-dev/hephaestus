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

    pub fn normalize(self) -> Self {
        Self(self.0.normalize())
    }
}

impl Default for Vec3 {
    fn default() -> Self {
        Self(glam::Vec3::ZERO)
    }
}

impl Quat {
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        Quat::from(glam::Quat::from_axis_angle(glam::Vec3::from(axis), angle))
    }
}

impl Default for Quat {
    fn default() -> Self {
        Self(glam::Quat::IDENTITY)
    }
}
