use crate::math::{Mat4, Quat, Vec3};

impl Mat4 {
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Mat4 {
        Mat4::from(glam::Mat4::from_rotation_translation(
            rotation.0,
            translation.0,
        ))
    }
}

impl std::ops::Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: Mat4) -> Mat4 {
        Mat4(self.0 * rhs.0)
    }
}
