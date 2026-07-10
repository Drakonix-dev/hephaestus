use crate::math::{Mat3, Mat4, Quat, Vec2, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            position: Vec2::default(),
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
        }
    }
}

impl Transform2D {
    pub fn matrix(&self) -> Mat3 {
        Mat3::from(glam::Mat3::from_scale_angle_translation(
            glam::Vec2::from(self.scale),
            self.rotation,
            glam::Vec2::from(self.position),
        ))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform3D {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            position: Vec3::default(),
            rotation: Quat::default(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform3D {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from(glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::from(self.scale),
            glam::Quat::from(self.rotation),
            glam::Vec3::from(self.position),
        ))
    }
}
