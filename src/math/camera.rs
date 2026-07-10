use crate::math::{Mat4, Vec3};

pub fn look_at_mat4(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    Mat4::from(glam::camera::rh::view::look_at_mat4(
        glam::Vec3::from(eye),
        glam::Vec3::from(center),
        glam::Vec3::from(up),
    ))
}

pub fn perspective(vertical_fov: f32, aspect_ratio: f32, near: f32, far: f32) -> Mat4 {
    Mat4::from(glam::camera::rh::proj::directx::perspective(
        vertical_fov,
        aspect_ratio,
        near,
        far,
    ))
}
