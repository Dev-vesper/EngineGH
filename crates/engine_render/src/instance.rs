use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};

/// Per-instance data for the mesh pipeline. `color` is linear RGBA.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Instance {
    pub model: Mat4,
    pub color: Vec4,
}

impl Instance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Instance>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, // model columns
            6 => Float32x4, // color
        ],
    };
}

/// GPU buffer that grows geometrically; rewritten each frame via `queue.write_buffer`.
pub struct DynamicBuffer {
    buffer: wgpu::Buffer,
    capacity: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl DynamicBuffer {
    pub fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages, capacity: u64) -> Self {
        let capacity = capacity.max(256).next_power_of_two();
        Self { buffer: Self::alloc(device, label, usage, capacity), capacity, usage, label }
    }

    fn alloc(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) {
        let needed = data.len() as u64;
        if needed > self.capacity {
            self.capacity = needed.next_power_of_two();
            self.buffer = Self::alloc(device, self.label, self.usage, self.capacity);
        }
        if needed > 0 {
            queue.write_buffer(&self.buffer, 0, data);
        }
    }

    #[inline]
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}
