use std::mem::size_of;

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl LineVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<LineVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };
}

const MINOR: [f32; 3] = [0.05, 0.05, 0.06];
const MAJOR: [f32; 3] = [0.12, 0.12, 0.14];
const AXIS_X: [f32; 3] = [0.9, 0.1, 0.1];
const AXIS_Y: [f32; 3] = [0.1, 0.9, 0.1];
const AXIS_Z: [f32; 3] = [0.1, 0.25, 0.95];

/// Line-list vertices for an XZ grid at y = 0 spanning `[-half, half]` with 1-unit spacing,
/// a brighter line every `major_every` units, plus coloured world axes (Y axis is `half / 10` tall).
pub fn build_grid(half: i32, major_every: i32) -> Vec<LineVertex> {
    let h = half as f32;
    let mut v = Vec::with_capacity(((half * 2 + 1) * 4 + 6) as usize);
    for i in -half..=half {
        if i == 0 {
            continue; // drawn as axes below
        }
        let color = if i % major_every == 0 { MAJOR } else { MINOR };
        let f = i as f32;
        v.push(LineVertex { position: [f, 0.0, -h], color });
        v.push(LineVertex { position: [f, 0.0, h], color });
        v.push(LineVertex { position: [-h, 0.0, f], color });
        v.push(LineVertex { position: [h, 0.0, f], color });
    }
    let axis = |a: [f32; 3], b: [f32; 3], color| [LineVertex { position: a, color }, LineVertex { position: b, color }];
    v.extend(axis([-h, 0.0, 0.0], [h, 0.0, 0.0], AXIS_X));
    v.extend(axis([0.0, 0.0, -h], [0.0, 0.0, h], AXIS_Z));
    v.extend(axis([0.0, 0.0, 0.0], [0.0, h / 10.0, 0.0], AXIS_Y));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_vertex_count_matches_capacity_formula() {
        let g = build_grid(100, 10);
        // 200 non-axis indices * 4 vertices + 3 axes * 2 vertices.
        assert_eq!(g.len(), 200 * 4 + 6);
        assert_eq!(g.len() % 2, 0);
    }
}
