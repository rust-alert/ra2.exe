//! 选中反馈标记：叠在预览图之上（选中环、生命条、路径点、攻击目标）。
//!
//! 单位本体由底图 VXL/SHP 表达，不再为每个实体画实心菱形/方块。
//! 绘制数据来自 [`crate::world::RenderWorld`]。

use bytemuck::{Pod, Zeroable};

use crate::{camera::Camera, world::RenderWorld};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

/// 最大同时绘制的标记顶点数（每单位 6 顶点，外加选中环）。
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

    /// 从可复用 [`RenderWorld`] 写入标记顶点（屏外粗裁剪，避免上传不可见单位）。
    ///
    /// 底图已叠 VXL/SHP 时，不再为每个实体画实心菱形/方块（会叠出「一堆无意义色块」）。
    /// 仅对**选中**实体画选中环、生命条、路径点与攻击目标标记。
    pub fn write_from_world(&mut self, queue: &wgpu::Queue, world: &RenderWorld, camera: &Camera, surface_w: u32, surface_h: u32) {
        let mut verts: Vec<Vertex> = Vec::new();
        let sw = surface_w.max(1) as f32;
        let sh = surface_h.max(1) as f32;
        const MARGIN: f32 = 1.15;
        for u in world.units.values().filter(|u| !u.dead && u.selected) {
            let cx = u.screen_x as f32 + 30.0;
            let cy = u.screen_y as f32 + 15.0;
            let unit_visible = ndc_visible(camera.world_to_ndc(cx, cy, sw, sh), MARGIN);
            let half = if u.is_structure { 12.0 } else { 10.0 };
            if unit_visible {
                if u.deployable {
                    // 可部署单位（MCV 等）：部署标记 = 青绿菱形底板 + 外环，区别于普通黄环。
                    let fill = [0.15, 0.85, 0.35, 0.55];
                    let ring = [0.25, 1.0, 0.45, 0.98];
                    push_diamond(&mut verts, camera, sw, sh, cx, cy, half + 2.0, fill);
                    push_ring(&mut verts, camera, sw, sh, cx, cy, half + 6.0, 2.5, ring);
                    push_ring(&mut verts, camera, sw, sh, cx, cy, half + 1.0, 1.5, [0.9, 1.0, 0.4, 0.9]);
                } else {
                    let ring = [1.0, 1.0, 0.2, 0.95];
                    push_ring(&mut verts, camera, sw, sh, cx, cy, half + 4.0, 2.0, ring);
                }
                if u.max_health > 0 {
                    let ratio = (u.health as f32 / u.max_health as f32).clamp(0.0, 1.0);
                    let bar_w = 18.0;
                    let bar_h = 3.0;
                    let bx = cx - bar_w * 0.5;
                    let by = cy - half - 6.0;
                    push_rect(&mut verts, camera, sw, sh, bx, by, bar_w, bar_h, [0.1, 0.1, 0.1, 0.85]);
                    push_rect(&mut verts, camera, sw, sh, bx, by, bar_w * ratio, bar_h, [0.2, 0.9, 0.25, 0.95]);
                }
            }
            // 路径点：浅绿小菱形；终点：稍大青绿菱形（单位离屏时仍画可见点）。
            let path_fill = [0.35, 0.95, 0.45, 0.75];
            let goal_fill = [0.2, 1.0, 0.4, 0.9];
            for &(px, py) in &u.path_waypoints_screen {
                let (px, py) = (px as f32, py as f32);
                if !ndc_visible(camera.world_to_ndc(px, py, sw, sh), MARGIN) {
                    continue;
                }
                push_diamond(&mut verts, camera, sw, sh, px, py, 4.0, path_fill);
                if verts.len() as u64 >= MAX_VERTICES {
                    break;
                }
            }
            if let Some((gx, gy)) = u.move_goal_screen {
                let (gx, gy) = (gx as f32, gy as f32);
                if ndc_visible(camera.world_to_ndc(gx, gy, sw, sh), MARGIN) {
                    push_diamond(&mut verts, camera, sw, sh, gx, gy, 7.0, goal_fill);
                    push_ring(&mut verts, camera, sw, sh, gx, gy, 9.0, 1.5, [0.4, 1.0, 0.55, 0.95]);
                }
            }
            // 攻击标记：红色交叉叠在目标锚点上。
            if let Some((ax, ay)) = u.attack_target_screen {
                let (ax, ay) = (ax as f32, ay as f32);
                if ndc_visible(camera.world_to_ndc(ax, ay, sw, sh), MARGIN) {
                    push_attack_mark(&mut verts, camera, sw, sh, ax, ay, [1.0, 0.2, 0.15, 0.95]);
                }
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

fn push_diamond(out: &mut Vec<Vertex>, camera: &Camera, sw: f32, sh: f32, cx: f32, cy: f32, half: f32, color: [f32; 4]) {
    let top = camera.world_to_ndc(cx, cy - half, sw, sh);
    let right = camera.world_to_ndc(cx + half, cy, sw, sh);
    let bottom = camera.world_to_ndc(cx, cy + half, sw, sh);
    let left = camera.world_to_ndc(cx - half, cy, sw, sh);
    out.extend_from_slice(&[
        Vertex { pos: top, color },
        Vertex { pos: right, color },
        Vertex { pos: bottom, color },
        Vertex { pos: top, color },
        Vertex { pos: bottom, color },
        Vertex { pos: left, color },
    ]);
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

/// 攻击标记：两条对角粗线组成的「X」。
fn push_attack_mark(out: &mut Vec<Vertex>, camera: &Camera, sw: f32, sh: f32, cx: f32, cy: f32, color: [f32; 4]) {
    const ARM: f32 = 10.0;
    const THICK: f32 = 2.5;
    push_line_quad(out, camera, sw, sh, cx - ARM, cy - ARM, cx + ARM, cy + ARM, THICK, color);
    push_line_quad(out, camera, sw, sh, cx + ARM, cy - ARM, cx - ARM, cy + ARM, THICK, color);
    push_ring(out, camera, sw, sh, cx, cy, 12.0, 1.5, color);
}

fn push_line_quad(
    out: &mut Vec<Vertex>,
    camera: &Camera,
    sw: f32,
    sh: f32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    thickness: f32,
    color: [f32; 4],
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt().max(1e-3);
    let nx = (-dy / len) * (thickness * 0.5);
    let ny = (dx / len) * (thickness * 0.5);
    let a = camera.world_to_ndc(x0 + nx, y0 + ny, sw, sh);
    let b = camera.world_to_ndc(x1 + nx, y1 + ny, sw, sh);
    let c = camera.world_to_ndc(x1 - nx, y1 - ny, sw, sh);
    let d = camera.world_to_ndc(x0 - nx, y0 - ny, sw, sh);
    out.extend_from_slice(&[
        Vertex { pos: a, color },
        Vertex { pos: b, color },
        Vertex { pos: c, color },
        Vertex { pos: a, color },
        Vertex { pos: c, color },
        Vertex { pos: d, color },
    ]);
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
