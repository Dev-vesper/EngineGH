use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl MeshVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<MeshVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };
}

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u16>,
}

/// Unit cube centred on the origin (edge length 1), CCW winding, per-face normals.
pub fn unit_cube() -> MeshData {
    use glam::Vec3;
    // (normal, u, v) with u x v == normal, so (-u-v, u-v, u+v, -u+v) is CCW seen from outside.
    const FACES: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::Y, Vec3::X),
    ];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (n, u, v) in FACES {
        let base = vertices.len() as u16;
        for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let p = (n + u * su + v * sv) * 0.5;
            vertices.push(MeshVertex { position: p.to_array(), normal: n.to_array() });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    MeshData { vertices, indices }
}

pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl GpuMesh {
    pub fn upload(device: &wgpu::Device, label: &str, data: &MeshData) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self { vertex_buffer, index_buffer, index_count: data.indices.len() as u32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn cube_triangles_wind_ccw_outward_and_match_normals() {
        let m = unit_cube();
        assert_eq!(m.vertices.len(), 24);
        assert_eq!(m.indices.len(), 36);
        for tri in m.indices.chunks_exact(3) {
            let v = |i: u16| Vec3::from(m.vertices[i as usize].position);
            let (a, b, c) = (v(tri[0]), v(tri[1]), v(tri[2]));
            let face_n = (b - a).cross(c - a).normalize();
            let stored_n = Vec3::from(m.vertices[tri[0] as usize].normal);
            assert!((face_n - stored_n).length() < 1e-5, "winding disagrees with normal");
            assert!(face_n.dot(a) > 0.0, "normal points inward");
        }
    }
}
