use crate::math::Mat4;

pub fn perspective(fovy: f32, aspect_ratio: f32, near: f32, far: f32) -> Mat4 {
    Mat4::from(glam::camera::rh::proj::directx::perspective(
        fovy,
        aspect_ratio,
        near,
        far,
    ))
}
