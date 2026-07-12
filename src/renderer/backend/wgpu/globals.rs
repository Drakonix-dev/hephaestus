use std::{mem, num::NonZeroU64};

use crate::{
    diagnostics::diag,
    math::Mat4,
    renderer::{FrameError, RenderError, Transform},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalsUniforms {
    view_proj: Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ModelUniforms {
    model: Mat4,
}

pub(crate) struct FrameGlobals {
    pub(crate) layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
}

impl FrameGlobals {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Globals Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(mem::size_of::<GlobalsUniforms>() as u64),
                },
                count: None,
            }],
        });

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Globals Uniform Buffer"),
            size: mem::size_of::<GlobalsUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Globals Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            layout,
            buffer,
            bind_group,
        }
    }

    pub(crate) fn write(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::bytes_of(&GlobalsUniforms { view_proj }),
        );
    }
}

pub(crate) struct ModelUniformPool {
    pub(crate) layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
    stride: u64,
    capacity: u64,
}

impl ModelUniformPool {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self::with_capacity(device, 1)
    }

    fn with_capacity(device: &wgpu::Device, capacity: u64) -> Self {
        let layout = Self::create_layout(device);

        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let unaligned = mem::size_of::<ModelUniforms>() as u64;
        let stride = unaligned.div_ceil(alignment) * alignment;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Model Uniform Buffer"),
            size: stride * capacity,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = Self::create_bind_group(device, &layout, &buffer, unaligned);

        Self {
            layout,
            buffer,
            bind_group,
            stride,
            capacity,
        }
    }

    fn create_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Model Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(mem::size_of::<ModelUniforms>() as u64),
                },
                count: None,
            }],
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
        slot_size: u64,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer,
                    offset: 0,
                    size: NonZeroU64::new(slot_size),
                }),
            }],
        })
    }

    pub(crate) fn reserve(&mut self, device: &wgpu::Device, draw_count: u64) {
        if draw_count <= self.capacity {
            return;
        }

        let mut new_capacity = self.capacity.max(1);
        while new_capacity < draw_count {
            new_capacity *= 2;
        }

        tracing::debug!(
            target: diag::UPLOAD,
            from = self.capacity,
            to = new_capacity,
            "model uniform pool grown",
        );

        *self = Self::with_capacity(device, new_capacity);
    }

    pub(crate) fn write(
        &self,
        queue: &wgpu::Queue,
        index: u64,
        transform: &Transform,
    ) -> Result<u32, RenderError> {
        if index >= self.capacity {
            tracing::warn!(
                target: diag::UPLOAD,
                draws = index + 1,
                capacity = self.capacity,
                "model uniform pool exhausted mid-frame",
            );
            return Err(RenderError::Frame(FrameError::Other(format!(
                "model uniform pool exhausted mid-frame ({} draws, capacity {}) - reserve() should have grown it first",
                index + 1,
                self.capacity
            ))));
        }

        let model = match transform {
            Transform::Transform3D(t) => t.matrix(),
            Transform::Transform2D(_) => Mat4::from(glam::Mat4::IDENTITY),
        };

        let offset = index * self.stride;
        queue.write_buffer(
            &self.buffer,
            offset,
            bytemuck::bytes_of(&ModelUniforms { model }),
        );

        Ok(offset as u32)
    }
}
