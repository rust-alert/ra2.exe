//! 屏上归一化色块（占位 HUD / 结算条），不走世界相机。
//!
//! **不是原版 HUD。** 仅证明对局页可叠一层固定 UI 几何；正式 SHP/字体另接。

use bytemuck::{Pod, Zeroable};

/// 窗口归一化矩形色块（左上为原点，`0..1`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenChromeQuad {
    /// 左。
    pub x0: f32,
    /// 上。
    pub y0: f32,
    /// 右。
    pub x1: f32,
    /// 下。
    pub y1: f32,
    /// RGBA（0..1）。
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

const MAX_VERTICES: u64 = 512 * 6;

/// 屏上色块 GPU 槽（与单位 marker 管线同构，顶点已是 NDC）。
pub struct ChromeGpu {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl ChromeGpu {
    /// 创建与 surface 格式匹配的色块管线。
    pub fn create(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ra.chrome.wgsl"),
            source: wgpu::ShaderSource::Wgsl(CHROME_WGSL.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ra.chrome.pl"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ra.chrome.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ra.chrome.vb"),
            size: MAX_VERTICES * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            vertex_buffer,
            vertex_count: 0,
        }
    }

    /// 用归一化矩形写入 NDC 顶点（Y 向下）。
    pub fn write_quads(&mut self, queue: &wgpu::Queue, quads: &[ScreenChromeQuad]) {
        let mut verts: Vec<Vertex> = Vec::with_capacity(quads.len() * 6);
        for q in quads {
            if verts.len() as u64 + 6 > MAX_VERTICES {
                break;
            }
            push_ndc_rect(&mut verts, q.x0, q.y0, q.x1, q.y1, q.color);
        }
        self.vertex_count = verts.len() as u32;
        if !verts.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
        }
    }

    /// 绘制已上传色块。
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }

    /// 清空本帧色块。
    pub fn clear(&mut self) {
        self.vertex_count = 0;
    }
}

fn push_ndc_rect(out: &mut Vec<Vertex>, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
    let p00 = norm_to_ndc(x0, y0);
    let p10 = norm_to_ndc(x1, y0);
    let p11 = norm_to_ndc(x1, y1);
    let p01 = norm_to_ndc(x0, y1);
    out.extend_from_slice(&[
        Vertex { pos: p00, color },
        Vertex { pos: p10, color },
        Vertex { pos: p11, color },
        Vertex { pos: p00, color },
        Vertex { pos: p11, color },
        Vertex { pos: p01, color },
    ]);
}

fn norm_to_ndc(nx: f32, ny: f32) -> [f32; 2] {
    [nx.mul_add(2.0, -1.0), 1.0 - ny * 2.0]
}

const CHROME_WGSL: &str = r#"
struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = vec4<f32>(v.pos, 0.0, 1.0);
    out.color = v.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_to_ndc_corners() {
        assert_eq!(norm_to_ndc(0.0, 0.0), [-1.0, 1.0]);
        assert_eq!(norm_to_ndc(1.0, 1.0), [1.0, -1.0]);
        assert_eq!(norm_to_ndc(0.5, 0.5), [0.0, 0.0]);
    }
}
