pub mod view;

use bytemuck::{Pod, Zeroable};
use math_derive::{WrapFrom, wrap_fns};

macro_rules! impl_default {
    ($name:ident, $d:expr) => {
        impl Default for $name {
            fn default() -> Self {
                Self($d)
            }
        }
    };
}

#[derive(WrapFrom, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Mat4(pub(crate) glam::Mat4);

wrap_fns!(Mat4, glam::Mat4, {
    fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self;
    fn from_scale_rotation_translation(scale: Vec3, rotation: Quat, translation: Vec3) -> Self;
    fn inverse(&self) -> Self;
});

impl std::ops::Mul for Mat4 {
    type Output = Mat4;
    fn mul(self, rhs: Mat4) -> Mat4 {
        Mat4(self.0 * rhs.0)
    }
}

#[derive(WrapFrom, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Quat(pub(crate) glam::Quat);

wrap_fns!(Quat, glam::Quat, {
    fn from_axis_angle(axis: Vec3, angle: f32) -> Self;
});
impl_default!(Quat, glam::Quat::IDENTITY);

#[derive(WrapFrom, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vec2(pub(crate) glam::Vec2);

wrap_fns!(Vec2, glam::Vec2, {
    fn new(x: f32, y: f32) -> Self;
});
impl_default!(Vec2, glam::Vec2::ZERO);

#[derive(WrapFrom, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vec3(pub(crate) glam::Vec3);

wrap_fns!(Vec3, glam::Vec3, {
    fn new(x: f32, y: f32, z: f32) -> Self;
    fn normalize(self) -> Self;
});
impl_default!(Vec3, glam::Vec3::ZERO);

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
        Mat4::from_scale_rotation_translation(
            self.scale.0.into(),
            self.rotation.0.into(),
            self.position.0.into(),
        )
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position).inverse()
    }
}
