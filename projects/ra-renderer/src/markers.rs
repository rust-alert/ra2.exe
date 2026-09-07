//! 纯色菱形/方块标记：叠在预览图之上，表达单位位置与选中。

use bytemuck::{Pod, Zeroable};

use ra_session::{AnimState, RenderSnapshot};

use crate::camera::Camera;

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

    pub fn write_from_snapshot(
        &mut self,
        queue: &wgpu::Queue,
        snap: &RenderSnapshot,
        camera: &Camera,
        surface_w: u32,
        surface_h: u32,
    ) {
        let mut verts: Vec<Vertex> = Vec::new();
        let sw = surface_w.max(1) as f32;
        let sh = surface_h.max(1) as f32;
        for u in snap.units.iter().filter(|u| !u.dead) {
            let selected = snap.selected.contains(&u.index);
            let color = anim_tint(owner_color(&u.owner), u.anim_state);
            let cx = u.screen_x as f32 + 30.0;
            let cy = u.screen_y as f32 + 15.0;
            let half = match (u.is_structure(), selected) {
                (true, true) => 12.0,
                (true, false) => 9.0,
                (false, true) => 10.0,
                (false, false) => 7.0,
            };
            if u.is_structure() {
                push_rect(
                    &mut verts,
                    camera,
                    sw,
                    sh,
                    cx - half,
                    cy - half * 0.6,
                    half * 2.0,
                    half * 1.2,
                    color,
                );
            }
            else {
                push_diamond(&mut verts, camera, sw, sh, cx, cy, half, color);
            }
            if selected {
                let ring = [1.0, 1.0, 0.2, 0.95];
                push_ring(&mut verts, camera, sw, sh, cx, cy, half + 4.0, 2.0, ring);
            }
            if u.max_health > 0 && u.health < u.max_health {
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

fn push_ring(
    out: &mut Vec<Vertex>,
    camera: &Camera,
    sw: f32,
    sh: f32,
    cx: f32,
    cy: f32,
    outer: f32,
    thickness: f32,
    color: [f32; 4],
) {
    // 简化：四个边框矩形近似环。
    let inner = outer - thickness;
    push_rect(out, camera, sw, sh, cx - outer, cy - outer, outer * 2.0, thickness, color);
    push_rect(out, camera, sw, sh, cx - outer, cy + inner, outer * 2.0, thickness, color);
    push_rect(out, camera, sw, sh, cx - outer, cy - inner, thickness, inner * 2.0, color);
    push_rect(out, camera, sw, sh, cx + inner, cy - inner, thickness, inner * 2.0, color);
}

fn owner_color(owner: &str) -> [f32; 4] {
    let mut h: u32 = 2166136261;
    for b in owner.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(16777619);
    }
    let r = ((h >> 16) & 0xff) as f32 / 255.0;
    let g = ((h >> 8) & 0xff) as f32 / 255.0;
    let b = (h & 0xff) as f32 / 255.0;
    [0.35 + r * 0.55, 0.35 + g * 0.55, 0.35 + b * 0.55, 0.92]
}

fn anim_tint(base: [f32; 4], state: AnimState) -> [f32; 4] {
    let (r, g, b, a) = (base[0], base[1], base[2], base[3]);
    match state {
        AnimState::Idle => base,
        AnimState::Move => [r * 0.85 + 0.15, g * 0.85 + 0.15, b * 0.7, a],
        AnimState::Attack => [r * 0.55 + 0.45, g * 0.45, b * 0.35, a],
        AnimState::TakeDamage => [0.95, 0.95, 0.95, a],
        AnimState::Produce => [r * 0.55, g * 0.55 + 0.4, b * 0.7 + 0.25, a],
        AnimState::Die => [0.2, 0.2, 0.2, 0.55],
    }
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
