//! 对局命令图标 GPU：`mouse.shp` 帧图集叠在预览图之上。
//!
//! 移动 / 攻击目标点、选中可部署单位处绘制原版光标序列，不自绘几何图标。

use bytemuck::{Pod, Zeroable};
use image::RgbaImage;

use crate::{camera::Camera, world::RenderWorld};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

/// 单类图标在图集中的帧区间。
#[derive(Debug, Clone, Copy)]
struct FrameRange {
    start: u32,
    count: u32,
}

/// 最大同时绘制的图标实例（每实例 6 顶点）。
const MAX_INSTANCES: u64 = 256;
const MAX_VERTICES: u64 = MAX_INSTANCES * 6;

/// CPU 侧解码后的命令图标（上传前）。
#[derive(Debug, Clone)]
pub struct DecodedOrderIcons {
    /// 单帧画布宽。
    pub canvas_w: u32,
    /// 单帧画布高。
    pub canvas_h: u32,
    /// Move 帧。
    pub move_frames: Vec<RgbaImage>,
    /// Attack 帧。
    pub attack_frames: Vec<RgbaImage>,
    /// Deploy 帧。
    pub deploy_frames: Vec<RgbaImage>,
}

/// `mouse.shp` 命令图标图集与绘制管线。
pub struct OrderIconGpu {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    frame_w: u32,
    frame_h: u32,
    atlas_cols: u32,
    atlas_w: u32,
    atlas_h: u32,
    move_range: FrameRange,
    attack_range: FrameRange,
    deploy_range: FrameRange,
}

impl OrderIconGpu {
    /// 将 CPU 帧打包为图集并创建管线。
    pub fn create(device: &wgpu::Device, queue: &wgpu::Queue, surface_format: wgpu::TextureFormat, icons: &DecodedOrderIcons) -> Option<Self> {
        let frame_w = icons.canvas_w.max(1);
        let frame_h = icons.canvas_h.max(1);
        let mut all: Vec<&RgbaImage> = Vec::new();
        let move_start = 0u32;
        for f in &icons.move_frames {
            all.push(f);
        }
        let move_count = icons.move_frames.len() as u32;
        let attack_start = move_count;
        for f in &icons.attack_frames {
            all.push(f);
        }
        let attack_count = icons.attack_frames.len() as u32;
        let deploy_start = attack_start + attack_count;
        for f in &icons.deploy_frames {
            all.push(f);
        }
        let deploy_count = icons.deploy_frames.len() as u32;
        if all.is_empty() {
            return None;
        }
        let n = all.len() as u32;
        let atlas_cols = n.min(8).max(1);
        let atlas_rows = n.div_ceil(atlas_cols);
        let atlas_w = atlas_cols * frame_w;
        let atlas_h = atlas_rows * frame_h;
        let mut atlas = RgbaImage::new(atlas_w, atlas_h);
        for (i, frame) in all.iter().enumerate() {
            let col = (i as u32) % atlas_cols;
            let row = (i as u32) / atlas_cols;
            let ox = col * frame_w;
            let oy = row * frame_h;
            blit_frame(&mut atlas, frame, ox, oy, frame_w, frame_h);
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ra.order_icon.wgsl"),
            source: wgpu::ShaderSource::Wgsl(ORDER_ICON_WGSL.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ra.order_icon.bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ra.order_icon.pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ra.order_icon.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ra.order_icon.sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let size = wgpu::Extent3d { width: atlas_w, height: atlas_h, depth_or_array_layers: 1 };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ra.order_icon.tex"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            atlas.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * atlas_w),
                rows_per_image: Some(atlas_h),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ra.order_icon.bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ra.order_icon.vb"),
            size: MAX_VERTICES * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Some(Self {
            pipeline,
            bind_group,
            vertex_buffer,
            vertex_count: 0,
            frame_w,
            frame_h,
            atlas_cols,
            atlas_w,
            atlas_h,
            move_range: FrameRange { start: move_start, count: move_count },
            attack_range: FrameRange { start: attack_start, count: attack_count },
            deploy_range: FrameRange { start: deploy_start, count: deploy_count },
        })
    }

    /// 写入本帧图标顶点（选中行动线窗口内画移动/攻击目标；选中可部署画部署帧）。
    pub fn write_from_world(&mut self, queue: &wgpu::Queue, world: &RenderWorld, camera: &Camera, surface_w: u32, surface_h: u32) {
        let mut verts: Vec<Vertex> = Vec::new();
        let sw = surface_w.max(1) as f32;
        let sh = surface_h.max(1) as f32;
        let tick = world.source_tick;
        for u in world.units.values().filter(|u| !u.dead && u.selected) {
            if u.deployable && self.deploy_range.count > 0 {
                let cx = u.screen_x as f32 + 30.0;
                let cy = u.screen_y as f32 + 15.0;
                let fi = self.deploy_range.start + (tick % u64::from(self.deploy_range.count.max(1))) as u32;
                push_icon_quad(&mut verts, camera, sw, sh, cx, cy, fi, self);
            }
            if world.action_lines_active && !u.is_structure {
                if let Some((ax, ay)) = u.attack_target_screen {
                    if self.attack_range.count > 0 {
                        let fi = self.attack_range.start + (tick % u64::from(self.attack_range.count.max(1))) as u32;
                        push_icon_quad(&mut verts, camera, sw, sh, ax as f32, ay as f32, fi, self);
                    }
                } else if let Some((gx, gy)) = u.move_goal_screen {
                    if self.move_range.count > 0 {
                        let fi = self.move_range.start + (tick % u64::from(self.move_range.count.max(1))) as u32;
                        push_icon_quad(&mut verts, camera, sw, sh, gx as f32, gy as f32, fi, self);
                    }
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

    /// 绘制已写入的图标。
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.vertex_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..self.vertex_count, 0..1);
    }

    /// 清空本帧。
    pub fn clear(&mut self) {
        self.vertex_count = 0;
    }
}

fn blit_frame(dst: &mut RgbaImage, src: &RgbaImage, ox: u32, oy: u32, fw: u32, fh: u32) {
    let copy_w = src.width().min(fw);
    let copy_h = src.height().min(fh);
    for y in 0..copy_h {
        for x in 0..copy_w {
            dst.put_pixel(ox + x, oy + y, *src.get_pixel(x, y));
        }
    }
}

fn push_icon_quad(out: &mut Vec<Vertex>, camera: &Camera, sw: f32, sh: f32, cx: f32, cy: f32, frame_index: u32, gpu: &OrderIconGpu) {
    let col = frame_index % gpu.atlas_cols;
    let row = frame_index / gpu.atlas_cols;
    let u0 = (col * gpu.frame_w) as f32 / gpu.atlas_w as f32;
    let v0 = (row * gpu.frame_h) as f32 / gpu.atlas_h as f32;
    let u1 = ((col + 1) * gpu.frame_w) as f32 / gpu.atlas_w as f32;
    let v1 = ((row + 1) * gpu.frame_h) as f32 / gpu.atlas_h as f32;
    // 光标热点为画布中心（CenterMiddle）。
    let x0 = cx - gpu.frame_w as f32 * 0.5;
    let y0 = cy - gpu.frame_h as f32 * 0.5;
    let x1 = x0 + gpu.frame_w as f32;
    let y1 = y0 + gpu.frame_h as f32;
    let p00 = camera.world_to_ndc(x0, y0, sw, sh);
    let p10 = camera.world_to_ndc(x1, y0, sw, sh);
    let p11 = camera.world_to_ndc(x1, y1, sw, sh);
    let p01 = camera.world_to_ndc(x0, y1, sw, sh);
    out.extend_from_slice(&[
        Vertex { pos: p00, uv: [u0, v0] },
        Vertex { pos: p10, uv: [u1, v0] },
        Vertex { pos: p11, uv: [u1, v1] },
        Vertex { pos: p00, uv: [u0, v0] },
        Vertex { pos: p11, uv: [u1, v1] },
        Vertex { pos: p01, uv: [u0, v1] },
    ]);
}

const ORDER_ICON_WGSL: &str = r#"
struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var icon_tex: texture_2d<f32>;
@group(0) @binding(1) var icon_samp: sampler;

@vertex
fn vs_main(v: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = vec4<f32>(v.pos, 0.0, 1.0);
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(icon_tex, icon_samp, in.uv);
}
"#;
