use crate::math::{Mat4, Quat, Vec3};

impl Mat4 {
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Mat4 {
        Mat4::from(glam::Mat4::from_rotation_translation(
            rotation.0,
            translation.0,
        ))
    }
}
