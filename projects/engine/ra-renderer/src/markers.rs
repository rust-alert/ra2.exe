//! 选中反馈标记：叠在预览图之上（选中环与生命条）。
//!
//! 移动 / 攻击 / 部署命令图标由 [`crate::order_icons::OrderIconGpu`] 绘制（`mouse.shp`）。
//! 绘制数据来自 [`crate::world::RenderWorld`]。

use bytemuck::{Pod, Zeroable};

use crate::{camera::Camera, world::RenderWorld};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

/// 最大同时绘制的标记顶点数。
const MAX_VERTICES: u64 = 4096 * 6;

pub struct MarkerGpu {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl MarkerGpu {
    pub fn create(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ra.marker.wgsl"),
            source: wgpu::ShaderSource::Wgsl(MARKER_WGSL.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ra.marker.pl"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ra.marker.pipeline"),
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
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ra.marker.vb"),
            size: MAX_VERTICES * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { pipeline, vertex_buffer, vertex_count: 0 }
    }

    /// 从可复用 [`RenderWorld`] 写入选中环与生命条。
    pub fn write_from_world(&mut self, queue: &wgpu::Queue, world: &RenderWorld, camera: &Camera, surface_w: u32, surface_h: u32) {
        let mut verts: Vec<Vertex> = Vec::new();
        let sw = surface_w.max(1) as f32;
        let sh = surface_h.max(1) as f32;
        const MARGIN: f32 = 1.15;
        for u in world.units.values().filter(|u| !u.dead && u.selected) {
            let cx = u.screen_x as f32 + 30.0;
            let cy = u.screen_y as f32 + 15.0;
            if !ndc_visible(camera.world_to_ndc(cx, cy, sw, sh), MARGIN) {
                continue;
            }
            let half = if u.is_structure { 12.0 } else { 10.0 };
            let ring = [1.0, 1.0, 0.2, 0.95];
            push_ring(&mut verts, camera, sw, sh, cx, cy, half + 4.0, 2.0, ring);
            if u.max_health > 0 {
                let ratio = (u.health as f32 / u.max_health as f32).clamp(0.0, 1.0);
                let bar_w = 18.0;
                let bar_h = 3.0;
                let bx = cx - bar_w * 0.5;
                let by = cy - half - 6.0;
                push_rect(&mut verts, camera, sw, sh, bx, by, bar_w, bar_h, [0.1, 0.1, 0.1, 0.85]);
                push_rect(&mut verts, camera, sw, sh, bx, by, bar_w * ratio, bar_h, [0.2, 0.9, 0.25, 0.95]);
            }
            if verts.len() as u64 >= MAX_VERTICES {
                break;
            }
        }
        self.vertex_count = verts.len() as u32;
        if !verts.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }

    pub fn clear(&mut self) {
        self.vertex_count = 0;
    }
}

fn push_rect(out: &mut Vec<Vertex>, camera: &Camera, sw: f32, sh: f32, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let p00 = camera.world_to_ndc(x, y, sw, sh);
    let p10 = camera.world_to_ndc(x + w, y, sw, sh);
    let p11 = camera.world_to_ndc(x + w, y + h, sw, sh);
    let p01 = camera.world_to_ndc(x, y + h, sw, sh);
    out.extend_from_slice(&[
        Vertex { pos: p00, color },
        Vertex { pos: p10, color },
        Vertex { pos: p11, color },
        Vertex { pos: p00, color },
        Vertex { pos: p11, color },
        Vertex { pos: p01, color },
    ]);
}

fn push_ring(out: &mut Vec<Vertex>, camera: &Camera, sw: f32, sh: f32, cx: f32, cy: f32, outer: f32, thickness: f32, color: [f32; 4]) {
    let inner = outer - thickness;
    push_rect(out, camera, sw, sh, cx - outer, cy - outer, outer * 2.0, thickness, color);
    push_rect(out, camera, sw, sh, cx - outer, cy + inner, outer * 2.0, thickness, color);
    push_rect(out, camera, sw, sh, cx - outer, cy - inner, thickness, inner * 2.0, color);
    push_rect(out, camera, sw, sh, cx + inner, cy - inner, thickness, inner * 2.0, color);
}

/// NDC 点是否在扩大后的可见窗内（粗裁剪 stub，非完整视锥）。
pub fn ndc_visible(ndc: [f32; 2], margin: f32) -> bool {
    ndc[0].abs() <= margin && ndc[1].abs() <= margin
}

const MARKER_WGSL: &str = r#"
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
