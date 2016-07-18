//! 选中 / 悬停血条与建筑括号：`pipbrd.shp` + `pips.shp`。
//!
//! 废除臆想黄环与连续绿条。单位底框用 `pipbrd`，血格用离散 pip；
//! 建筑为等距屋顶 L 角折线 + NW 边 pip（含空格）。

use bytemuck::{Pod, Zeroable};
use image::RgbaImage;

use crate::{camera::Camera, world::RenderUnit, world::RenderWorld};

/// NDC 粗裁剪。
pub fn ndc_visible(ndc: [f32; 2], margin: f32) -> bool {
    ndc[0].abs() <= margin && ndc[1].abs() <= margin
}

/// 载具 / 飞机最大血 pip 数。
pub const UNIT_PIPS_VEHICLE: u32 = 17;
/// 步兵最大血 pip 数。
pub const UNIT_PIPS_INFANTRY: u32 = 8;
/// 单位水平 pip 步进。
pub const UNIT_PIP_STEP_X: f32 = 2.0;
/// 建筑 NW 边 pip 步进。
pub const BUILDING_PIP_STEP_X: f32 = -4.0;
/// 建筑 NW 边 pip 步进 Y。
pub const BUILDING_PIP_STEP_Y: f32 = 2.0;
/// art `Height` → 屏幕像素。
pub const BUILDING_HEIGHT_PX: f32 = 15.0;

/// 库存 `pips.shp` 单位血格帧（绿 / 黄 / 红；RA2 `conquer.mix` 实测 15/16/17）。
pub const UNIT_PIP_FRAMES: [usize; 3] = [15, 16, 17];
/// 建筑血格：空 / 绿 / 黄 / 红。
pub const BUILDING_PIP_FRAMES: [usize; 4] = [0, 1, 2, 4];
/// `pips.shp` 至少帧数（需覆盖单位帧 17）。
pub const PIPS_MIN_FRAMES: usize = 18;
/// `pipbrd.shp` 至少帧数（载具 + 步兵）。
pub const PIPBRD_MIN_FRAMES: usize = 2;

/// CPU 侧解码后的选中血条素材。
#[derive(Debug, Clone)]
pub struct DecodedSelectionOverlay {
    /// `pipbrd.shp` 帧（期望 2：载具、步兵）。
    pub pipbrd_frames: Vec<RgbaImage>,
    /// `pips.shp` 全帧（按索引取用）。
    pub pips_frames: Vec<RgbaImage>,
    /// `pipbrd` 画布宽。
    pub pipbrd_canvas_w: u32,
    /// `pipbrd` 画布高。
    pub pipbrd_canvas_h: u32,
    /// `pips` 画布宽。
    pub pips_canvas_w: u32,
    /// `pips` 画布高。
    pub pips_canvas_h: u32,
}

/// 血色档位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthPipTone {
    /// 绿。
    Green,
    /// 黄。
    Yellow,
    /// 红。
    Red,
}

/// 按 `ConditionYellow` / `ConditionRed` 选色。
pub fn health_pip_tone(ratio: f32, yellow: f32, red: f32) -> HealthPipTone {
    if ratio > yellow {
        HealthPipTone::Green
    } else if ratio > red {
        HealthPipTone::Yellow
    } else {
        HealthPipTone::Red
    }
}

/// 填充 pip 数：`floor(ratio * max)`，存活时至少 1。
pub fn filled_pip_count(health: u32, max_health: u32, max_pips: u32) -> u32 {
    if max_pips == 0 || max_health == 0 || health == 0 {
        return 0;
    }
    let ratio = (health as f32 / max_health as f32).clamp(0.0, 1.0);
    ((ratio * max_pips as f32) as u32).max(1).min(max_pips)
}

/// 建筑血 pip 段数：`(foundation_h * 15) / 2`。
pub fn building_pip_count(foundation_h: u16) -> u32 {
    (u32::from(foundation_h) * 15) / 2
}

/// 选中画底框/括号；悬停未选中只画 pip。
pub fn status_visibility(selected: bool, hovered: bool) -> (bool, bool) {
    if selected {
        (true, true)
    } else if hovered {
        (false, true)
    } else {
        (false, false)
    }
}

/// 菱形格心（相对预览图）。
pub fn cell_center(screen_x: i32, screen_y: i32) -> (f32, f32) {
    (screen_x as f32 + 30.0, screen_y as f32 + 15.0)
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineVertex {
    pos: [f32; 2],
    color: [f32; 4],
}

const MAX_SPRITE_VERTICES: u64 = 8192 * 6;
const MAX_LINE_VERTICES: u64 = 8192 * 6;

#[derive(Clone, Copy)]
struct AtlasSlot {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    w: f32,
    h: f32,
}

/// 选中血条 GPU：SHP 图集 + 建筑折线。
pub struct MarkerGpu {
    sprite_pipeline: wgpu::RenderPipeline,
    sprite_bind_group: wgpu::BindGroup,
    sprite_vb: wgpu::Buffer,
    sprite_count: u32,
    line_pipeline: wgpu::RenderPipeline,
    line_vb: wgpu::Buffer,
    line_count: u32,
    pipbrd_slots: Vec<AtlasSlot>,
    pips_slots: Vec<AtlasSlot>,
}

impl MarkerGpu {
    /// 从图集创建管线；素材不足时返回 `None`。
    pub fn create(device: &wgpu::Device, queue: &wgpu::Queue, surface_format: wgpu::TextureFormat, assets: &DecodedSelectionOverlay) -> Option<Self> {
        if assets.pipbrd_frames.len() < PIPBRD_MIN_FRAMES || assets.pips_frames.len() < PIPS_MIN_FRAMES {
            return None;
        }
        let mut frames: Vec<&RgbaImage> = Vec::new();
        for f in &assets.pipbrd_frames {
            frames.push(f);
        }
        let pipbrd_count = assets.pipbrd_frames.len();
        for f in &assets.pips_frames {
            frames.push(f);
        }
        // 统一格尺寸便于图集打包；UV / 绘制尺寸仍用各帧自身画布（对齐 DrawSHP 居中）。
        let cell_w = frames.iter().map(|f| f.width().max(1)).max().unwrap_or(1);
        let cell_h = frames.iter().map(|f| f.height().max(1)).max().unwrap_or(1);
        let n = frames.len() as u32;
        let cols = n.min(8).max(1);
        let rows = n.div_ceil(cols);
        let atlas_w = cols * cell_w;
        let atlas_h = rows * cell_h;
        let mut atlas = RgbaImage::new(atlas_w, atlas_h);
        let mut slots = Vec::with_capacity(frames.len());
        for (i, frame) in frames.iter().enumerate() {
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            let ox = col * cell_w;
            let oy = row * cell_h;
            let fw = frame.width().max(1);
            let fh = frame.height().max(1);
            let cx = (cell_w.saturating_sub(fw)) / 2;
            let cy = (cell_h.saturating_sub(fh)) / 2;
            blit_at(&mut atlas, frame, ox + cx, oy + cy);
            let u0 = (ox + cx) as f32 / atlas_w as f32;
            let v0 = (oy + cy) as f32 / atlas_h as f32;
            let u1 = (ox + cx + fw) as f32 / atlas_w as f32;
            let v1 = (oy + cy + fh) as f32 / atlas_h as f32;
            slots.push(AtlasSlot {
                u0,
                v0,
                u1,
                v1,
                w: fw as f32,
                h: fh as f32,
            });
        }
        let pipbrd_slots = slots[..pipbrd_count].to_vec();
        let pips_slots = slots[pipbrd_count..].to_vec();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ra.selection.atlas"),
            size: wgpu::Extent3d {
                width: atlas_w,
                height: atlas_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * atlas_w),
                rows_per_image: Some(atlas_h),
            },
            wgpu::Extent3d {
                width: atlas_w,
                height: atlas_h,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ra.selection.samp"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let sprite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ra.selection.sprite.bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        let sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ra.selection.sprite.bg"),
            layout: &sprite_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let sprite_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ra.selection.sprite.wgsl"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_WGSL.into()),
        });
        let sprite_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ra.selection.sprite.pl"),
            bind_group_layouts: &[Some(&sprite_bgl)],
            immediate_size: 0,
        });
        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ra.selection.sprite.pipeline"),
            layout: Some(&sprite_pl),
            vertex: wgpu::VertexState {
                module: &sprite_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sprite_shader,
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

        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ra.selection.line.wgsl"),
            source: wgpu::ShaderSource::Wgsl(LINE_WGSL.into()),
        });
        let line_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ra.selection.line.pl"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ra.selection.line.pipeline"),
            layout: Some(&line_pl),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
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

        let sprite_vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ra.selection.sprite.vb"),
            size: MAX_SPRITE_VERTICES * std::mem::size_of::<SpriteVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let line_vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ra.selection.line.vb"),
            size: MAX_LINE_VERTICES * std::mem::size_of::<LineVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Some(Self {
            sprite_pipeline,
            sprite_bind_group,
            sprite_vb,
            sprite_count: 0,
            line_pipeline,
            line_vb,
            line_count: 0,
            pipbrd_slots,
            pips_slots,
        })
    }

    /// 从 [`RenderWorld`] 写入本帧选中 / 悬停血条。
    pub fn write_from_world(&mut self, queue: &wgpu::Queue, world: &RenderWorld, camera: &Camera, surface_w: u32, surface_h: u32) {
        let mut sprites: Vec<SpriteVertex> = Vec::new();
        let mut lines: Vec<LineVertex> = Vec::new();
        let sw = surface_w.max(1) as f32;
        let sh = surface_h.max(1) as f32;
        const MARGIN: f32 = 1.25;
        let yellow = if world.condition_yellow > 0.0 {
            world.condition_yellow
        } else {
            0.5
        };
        let red = if world.condition_red > 0.0 {
            world.condition_red
        } else {
            0.25
        };

        for u in world.units.values().filter(|u| !u.dead) {
            let (draw_bracket, draw_pips) = status_visibility(u.selected, u.hovered);
            if !draw_bracket && !draw_pips {
                continue;
            }
            let (cx, cy) = cell_center(u.screen_x, u.screen_y);
            if !ndc_visible(camera.world_to_ndc(cx, cy, sw, sh), MARGIN) {
                continue;
            }
            let ratio = if u.max_health == 0 {
                0.0
            } else {
                (u.health as f32 / u.max_health as f32).clamp(0.0, 1.0)
            };
            let tone = health_pip_tone(ratio, yellow, red);

            if u.is_structure {
                if draw_bracket {
                    push_building_brackets(&mut lines, camera, sw, sh, cx, cy, u);
                }
                if draw_pips {
                    push_building_pips(&mut sprites, camera, sw, sh, cx, cy, u, tone, &self.pips_slots);
                }
            } else {
                let infantry = u.is_infantry;
                if draw_bracket {
                    push_unit_pipbrd(&mut sprites, camera, sw, sh, cx, cy, u, infantry, &self.pipbrd_slots);
                }
                if draw_pips {
                    push_unit_pips(&mut sprites, camera, sw, sh, cx, cy, u, infantry, tone, &self.pips_slots);
                }
            }
            if sprites.len() as u64 >= MAX_SPRITE_VERTICES || lines.len() as u64 >= MAX_LINE_VERTICES {
                break;
            }
        }

        self.sprite_count = sprites.len() as u32;
        self.line_count = lines.len() as u32;
        if !sprites.is_empty() {
            queue.write_buffer(&self.sprite_vb, 0, bytemuck::cast_slice(&sprites));
        }
        if !lines.is_empty() {
            queue.write_buffer(&self.line_vb, 0, bytemuck::cast_slice(&lines));
        }
    }

    /// 绘制血条精灵与建筑括号。
    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.line_count > 0 {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.line_vb.slice(..));
            pass.draw(0..self.line_count, 0..1);
        }
        if self.sprite_count > 0 {
            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_bind_group(0, &self.sprite_bind_group, &[]);
            pass.set_vertex_buffer(0, self.sprite_vb.slice(..));
            pass.draw(0..self.sprite_count, 0..1);
        }
    }

    /// 清空本帧。
    pub fn clear(&mut self) {
        self.sprite_count = 0;
        self.line_count = 0;
    }
}

fn blit_at(dst: &mut RgbaImage, src: &RgbaImage, ox: u32, oy: u32) {
    for y in 0..src.height() {
        for x in 0..src.width() {
            let dx = ox + x;
            let dy = oy + y;
            if dx < dst.width() && dy < dst.height() {
                dst.put_pixel(dx, dy, *src.get_pixel(x, y));
            }
        }
    }
}

fn push_sprite_quad(
    out: &mut Vec<SpriteVertex>,
    camera: &Camera,
    sw: f32,
    sh: f32,
    cx: f32,
    cy: f32,
    slot: &AtlasSlot,
) {
    // 画布居中（对齐 DrawSHP 居中旗标）。
    let x0 = cx - slot.w * 0.5;
    let y0 = cy - slot.h * 0.5;
    let x1 = x0 + slot.w;
    let y1 = y0 + slot.h;
    let p00 = camera.world_to_ndc(x0, y0, sw, sh);
    let p10 = camera.world_to_ndc(x1, y0, sw, sh);
    let p11 = camera.world_to_ndc(x1, y1, sw, sh);
    let p01 = camera.world_to_ndc(x0, y1, sw, sh);
    out.extend_from_slice(&[
        SpriteVertex {
            pos: p00,
            uv: [slot.u0, slot.v0],
        },
        SpriteVertex {
            pos: p10,
            uv: [slot.u1, slot.v0],
        },
        SpriteVertex {
            pos: p11,
            uv: [slot.u1, slot.v1],
        },
        SpriteVertex {
            pos: p00,
            uv: [slot.u0, slot.v0],
        },
        SpriteVertex {
            pos: p11,
            uv: [slot.u1, slot.v1],
        },
        SpriteVertex {
            pos: p01,
            uv: [slot.u0, slot.v1],
        },
    ]);
}

fn push_unit_pipbrd(
    out: &mut Vec<SpriteVertex>,
    camera: &Camera,
    sw: f32,
    sh: f32,
    cx: f32,
    cy: f32,
    u: &RenderUnit,
    infantry: bool,
    slots: &[AtlasSlot],
) {
    let slot = if infantry {
        slots.get(1)
    } else {
        slots.first()
    };
    let Some(slot) = slot
    else {
        return;
    };
    let delta = u.bracket_delta as f32;
    let (ox, oy) = if infantry {
        (11.0, delta - 25.0)
    } else {
        (1.0, delta - 26.0)
    };
    push_sprite_quad(out, camera, sw, sh, cx + ox, cy + oy, slot);
}

fn push_unit_pips(
    out: &mut Vec<SpriteVertex>,
    camera: &Camera,
    sw: f32,
    sh: f32,
    cx: f32,
    cy: f32,
    u: &RenderUnit,
    infantry: bool,
    tone: HealthPipTone,
    pips: &[AtlasSlot],
) {
    let max_pips = if infantry {
        UNIT_PIPS_INFANTRY
    } else {
        UNIT_PIPS_VEHICLE
    };
    let filled = filled_pip_count(u.health, u.max_health, max_pips);
    if filled == 0 {
        return;
    }
    let frame = match tone {
        HealthPipTone::Green => UNIT_PIP_FRAMES[0],
        HealthPipTone::Yellow => UNIT_PIP_FRAMES[1],
        HealthPipTone::Red => UNIT_PIP_FRAMES[2],
    };
    let Some(slot) = pips.get(frame)
    else {
        return;
    };
    let delta = u.bracket_delta as f32;
    let (start_x, start_y) = if infantry {
        (-5.0, delta - 24.0)
    } else {
        (-15.0, delta - 25.0)
    };
    for i in 0..filled {
        let px = cx + start_x + i as f32 * UNIT_PIP_STEP_X;
        let py = cy + start_y;
        push_sprite_quad(out, camera, sw, sh, px, py, slot);
    }
}

fn push_building_pips(
    out: &mut Vec<SpriteVertex>,
    camera: &Camera,
    sw: f32,
    sh: f32,
    cx: f32,
    cy: f32,
    u: &RenderUnit,
    tone: HealthPipTone,
    pips: &[AtlasSlot],
) {
    let fw = u.foundation_w.max(1) as f32;
    let fh = u.foundation_h.max(1) as f32;
    let num = building_pip_count(u.foundation_h.max(1));
    if num == 0 {
        return;
    }
    let art_h = u.art_height.max(1) as f32;
    let n = num as f32;
    let foundation_center_x = cx + (fw - fh) * 15.0;
    let foundation_center_y = cy + (fw + fh) * 7.5 - 15.0;
    let projected_x = -(fw + fh) * 15.0;
    let projected_y = (fh - fw) * 7.5 - art_h * BUILDING_HEIGHT_PX;
    let start_x = foundation_center_x + projected_x + 3.0 + n * 4.0;
    let start_y = foundation_center_y + projected_y + 4.0 - n * 2.0;
    let filled = filled_pip_count(u.health, u.max_health, num);
    let fill_frame = match tone {
        HealthPipTone::Green => BUILDING_PIP_FRAMES[1],
        HealthPipTone::Yellow => BUILDING_PIP_FRAMES[2],
        HealthPipTone::Red => BUILDING_PIP_FRAMES[3],
    };
    let empty = pips.get(BUILDING_PIP_FRAMES[0]);
    let filled_slot = pips.get(fill_frame);
    for i in 0..num {
        let px = start_x + i as f32 * BUILDING_PIP_STEP_X;
        let py = start_y + i as f32 * BUILDING_PIP_STEP_Y;
        let slot = if i < filled {
            filled_slot
        } else {
            empty
        };
        if let Some(slot) = slot {
            push_sprite_quad(out, camera, sw, sh, px, py, slot);
        }
    }
}

fn push_building_brackets(out: &mut Vec<LineVertex>, camera: &Camera, sw: f32, sh: f32, sx: f32, sy: f32, u: &RenderUnit) {
    let fw = u.foundation_w.max(1) as f32;
    let fh = u.foundation_h.max(1) as f32;
    let z_screen = u.art_height.max(1) as f32 * BUILDING_HEIGHT_PX;
    let (ground, roof) = building_box_corners(sx, sy, fw, fh, z_screen);
    // 可见屋顶三角：BL / FL / BR，各向邻边伸出 25% 短 stub。
    let color = [1.0, 1.0, 1.0, 0.95];
    // BL roof
    push_stub_line(out, camera, sw, sh, roof[2], ground[2], color);
    push_stub_line(out, camera, sw, sh, roof[2], roof[0], color);
    push_stub_line(out, camera, sw, sh, roof[2], roof[3], color);
    // FL roof
    push_stub_line(out, camera, sw, sh, roof[0], ground[0], color);
    push_stub_line(out, camera, sw, sh, roof[0], roof[1], color);
    // BR roof
    push_stub_line(out, camera, sw, sh, roof[3], ground[3], color);
    push_stub_line(out, camera, sw, sh, roof[3], roof[1], color);
}

#[derive(Clone, Copy)]
struct Pt {
    x: f32,
    y: f32,
}

fn building_box_corners(sx: f32, sy: f32, fw: f32, fh: f32, z_screen: f32) -> ([Pt; 4], [Pt; 4]) {
    let cx = sx + (fw - fh) * 15.0;
    let cy = sy + (fw + fh) * 7.5 - 15.0;
    let ground = [
        Pt {
            x: cx - (fw + fh) * 15.0,
            y: cy + (fh - fw) * 7.5,
        },
        Pt {
            x: cx + (fw - fh) * 15.0,
            y: cy + (fw + fh) * 7.5,
        },
        Pt {
            x: cx + (fh - fw) * 15.0,
            y: cy - (fw + fh) * 7.5,
        },
        Pt {
            x: cx + (fw + fh) * 15.0,
            y: cy - (fh - fw) * 7.5,
        },
    ];
    let roof = [
        Pt {
            x: ground[0].x,
            y: ground[0].y - z_screen,
        },
        Pt {
            x: ground[1].x,
            y: ground[1].y - z_screen,
        },
        Pt {
            x: ground[2].x,
            y: ground[2].y - z_screen,
        },
        Pt {
            x: ground[3].x,
            y: ground[3].y - z_screen,
        },
    ];
    (ground, roof)
}

fn quarter(a: Pt, b: Pt) -> Pt {
    Pt {
        x: ((a.x * 3.0 + b.x) * 0.25).trunc(),
        y: ((a.y * 3.0 + b.y) * 0.25).trunc(),
    }
}

fn push_stub_line(out: &mut Vec<LineVertex>, camera: &Camera, sw: f32, sh: f32, a: Pt, b: Pt, color: [f32; 4]) {
    let q = quarter(a, b);
    push_pixel_line(out, camera, sw, sh, a, q, color);
}

fn push_pixel_line(out: &mut Vec<LineVertex>, camera: &Camera, sw: f32, sh: f32, a: Pt, b: Pt, color: [f32; 4]) {
    let (mut x0, mut y0) = (a.x.trunc() as i32, a.y.trunc() as i32);
    let (mut x1, mut y1) = (b.x.trunc() as i32, b.y.trunc() as i32);
    if x0 > x1 {
        // 与原版一致：较小 X 端为起点。
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }
    draw_bresenham(out, camera, sw, sh, x0, y0, x1, y1, color);
}

fn draw_bresenham(
    out: &mut Vec<LineVertex>,
    camera: &Camera,
    sw: f32,
    sh: f32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [f32; 4],
) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x == x1 && y == y1 {
            break;
        }
        push_pixel_rect(out, camera, sw, sh, x as f32, y as f32, color);
        let e2 = err * 2;
        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
        if out.len() as u64 >= MAX_LINE_VERTICES {
            break;
        }
    }
}

fn push_pixel_rect(out: &mut Vec<LineVertex>, camera: &Camera, sw: f32, sh: f32, x: f32, y: f32, color: [f32; 4]) {
    let p00 = camera.world_to_ndc(x, y, sw, sh);
    let p10 = camera.world_to_ndc(x + 1.0, y, sw, sh);
    let p11 = camera.world_to_ndc(x + 1.0, y + 1.0, sw, sh);
    let p01 = camera.world_to_ndc(x, y + 1.0, sw, sh);
    out.extend_from_slice(&[
        LineVertex { pos: p00, color },
        LineVertex { pos: p10, color },
        LineVertex { pos: p11, color },
        LineVertex { pos: p00, color },
        LineVertex { pos: p11, color },
        LineVertex { pos: p01, color },
    ]);
}

const SPRITE_WGSL: &str = r#"
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
    let c = textureSample(icon_tex, icon_samp, in.uv);
    if (c.a < 0.01) { discard; }
    return c;
}
"#;

const LINE_WGSL: &str = r#"
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
    fn tone_thresholds_match_audio_visual_defaults() {
        assert_eq!(health_pip_tone(1.0, 0.5, 0.25), HealthPipTone::Green);
        assert_eq!(health_pip_tone(0.5, 0.5, 0.25), HealthPipTone::Yellow);
        assert_eq!(health_pip_tone(0.25, 0.5, 0.25), HealthPipTone::Red);
        assert_eq!(health_pip_tone(0.1, 0.5, 0.25), HealthPipTone::Red);
    }

    #[test]
    fn filled_pips_floor_and_min_one_while_alive() {
        assert_eq!(filled_pip_count(100, 100, 17), 17);
        assert_eq!(filled_pip_count(1, 100, 17), 1);
        assert_eq!(filled_pip_count(0, 100, 17), 0);
        assert_eq!(filled_pip_count(50, 100, 8), 4);
    }

    #[test]
    fn building_pip_count_by_foundation_height() {
        assert_eq!(building_pip_count(1), 7);
        assert_eq!(building_pip_count(2), 15);
        assert_eq!(building_pip_count(4), 30);
    }

    #[test]
    fn hover_skips_bracket_selected_keeps_both() {
        assert_eq!(status_visibility(true, false), (true, true));
        assert_eq!(status_visibility(false, true), (false, true));
        assert_eq!(status_visibility(false, false), (false, false));
    }
}
