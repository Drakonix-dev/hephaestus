use crate::math::Mat4;

pub fn perspective(fovy: f32, aspect_ratio: f32, near: f32, far: f32) -> Mat4 {
    glam::camera::rh::proj::directx::perspective(fovy, aspect_ratio, near, far).into()
}

pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    glam::camera::rh::proj::directx::orthographic(left, right, bottom, top, near, far).into()
}
