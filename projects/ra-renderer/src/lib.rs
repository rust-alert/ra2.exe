//! 读取呈现投影，经现代 GPU（wgpu）绘制。
//!
//! 原生后端：DX12 / Vulkan / Metal。Wasm：WebGL2。
//! 本 crate **故意不**实现 DirectDraw。
//!
//! # 架构阶段
//!
//! 当前仍是**快照驱动的 GPU 原型**（预览底图 + marker）。目标分层见模块
//! [`world`] / [`frame`] / [`resources`] / [`pass`] / [`timings`]：
//! `RenderWorld` → `FrameBuilder` → `PassGraph` → GPU batches。
//! 「使用 wgpu」不等于已解决原版 CPU 软件合成卡顿；禁止在本 `Renderer` 上无限堆临时绘制函数当作完成。

#![deny(missing_docs)]

mod camera;
mod frame;
mod gpu;
mod markers;
mod pass;
mod resources;
mod rgba_image;
mod sprite;
mod timings;
mod world;

use std::sync::Arc;

use ra_engine::RenderSnapshot;
use ra_types::{GameEdition, RaResult};
use winit::window::Window;

use crate::camera::Camera;
use crate::gpu::GpuContext;
use crate::markers::MarkerGpu;
use crate::sprite::SpriteGpu;

/// 2D 视口相机：平移与缩放，供外部读取或调整视角。
pub use crate::camera::Camera as ViewCamera;
/// 帧构建器（投影 → `RenderWorld`）。
pub use crate::frame::FrameBuilder;
/// 渲染阶段图。
pub use crate::pass::{PassGraph, RenderPassKind};
/// GPU 资源缓存骨架。
pub use crate::resources::RenderResourceCache;
/// CPU 侧 RGBA 像素缓冲，可上传到 GPU 作为预览纹理。
pub use crate::rgba_image::RgbaImage;
/// 帧分段计时。
pub use crate::timings::FrameTimings;
/// 可复用渲染世界。
pub use crate::world::RenderWorld;

/// 清屏底色（接近夜间战术图感觉，非最终主题）。
const CLEAR_COLOR: wgpu::Color = wgpu::Color { r: 0.04, g: 0.06, b: 0.09, a: 1.0 };

/// wgpu 渲染器：管理 surface 提交；长期应委托 `FrameBuilder` / `PassGraph`，而非堆砌临时 draw。
pub struct Renderer {
    /// 累计已提交帧数（含无 GPU 时的空转计数）。
    pub frames: u64,
    /// 最近一帧分段计时（由壳层可写入 simulation 段）。
    pub timings: FrameTimings,
    gpu: Option<GpuContext>,
    preview: Option<RgbaImage>,
    sprite: Option<SpriteGpu>,
    markers: Option<MarkerGpu>,
    camera: Camera,
    camera_ready: bool,
    /// 跨帧复用的渲染世界（R1）。
    render_world: RenderWorld,
    /// GPU 资源缓存（R1 骨架）。
    resources: RenderResourceCache,
    /// 阶段图（R1）。
    passes: PassGraph,
    frame_builder: FrameBuilder,
}

impl Renderer {
    /// 创建尚未绑定窗口的渲染器实例。
    pub fn new() -> Self {
        Self {
            frames: 0,
            timings: FrameTimings::default(),
            gpu: None,
            preview: None,
            sprite: None,
            markers: None,
            camera: Camera { center_x: 0.0, center_y: 0.0, zoom: 1.0 },
            camera_ready: false,
            render_world: RenderWorld::default(),
            resources: RenderResourceCache::default(),
            passes: PassGraph::prototype_default(),
            frame_builder: FrameBuilder,
        }
    }

    /// 设置启动预览图（窗口附着后上传）。
    pub fn set_preview(&mut self, image: RgbaImage) {
        if let Some(gpu) = self.gpu.as_ref() {
            match self.sprite.as_mut() {
                Some(sprite) => sprite.replace_image(&gpu.device, &gpu.queue, &image),
                None => {
                    self.sprite = Some(SpriteGpu::create(&gpu.device, &gpu.queue, gpu.config.format, &image));
                }
            }
            if self.markers.is_none() {
                self.markers = Some(MarkerGpu::create(&gpu.device, gpu.config.format));
            }
            self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width, image.height);
        }
        else {
            self.camera_ready = false;
        }
        self.preview = Some(image);
    }

    /// 窗口就绪后绑定表面。可重复调用（忽略已绑定）。
    pub fn attach_window(&mut self, window: Arc<Window>) -> RaResult<()> {
        if self.gpu.is_some() {
            return Ok(());
        }
        let gpu = GpuContext::new(window)?;
        if let Some(image) = self.preview.as_ref() {
            self.sprite = Some(SpriteGpu::create(&gpu.device, &gpu.queue, gpu.config.format, image));
            self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width, image.height);
        }
        self.markers = Some(MarkerGpu::create(&gpu.device, gpu.config.format));
        self.gpu = Some(gpu);
        Ok(())
    }

    /// 通知交换链表面尺寸变化（像素宽高）。
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.resize(width, height);
        }
    }

    /// 只读访问当前视口相机。
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// 可变访问当前视口相机。
    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    /// 按屏幕像素位移平移视口（拖拽地图）。
    pub fn pan_screen(&mut self, dx: f32, dy: f32) {
        self.camera.pan_screen(dx, dy);
    }

    /// 相对缩放视口，`factor` 大于 1 为放大。
    pub fn zoom_by(&mut self, factor: f32) {
        self.camera.zoom_by(factor);
    }

    fn reset_camera_to_fit(&mut self, screen_w: u32, screen_h: u32, image_w: u32, image_h: u32) {
        self.camera = Camera::fit(image_w, image_h, screen_w, screen_h);
        self.camera_ready = true;
    }

    /// 清屏：预览底图 + 快照单位标记（原型路径）。
    ///
    /// 先经 [`FrameBuilder`] 更新 [`RenderWorld`]，再走过渡期 sprite/marker 绘制。
    /// 完整场景应扩展 [`PassGraph`] 与 instance batch，而不是在此继续堆临时绘制分支。
    pub fn draw_frame(&mut self, snap: Option<&RenderSnapshot>) {
        self.frames = self.frames.wrapping_add(1);
        self.timings.clear();

        if let Some(snap) = snap {
            let build_start = std::time::Instant::now();
            FrameBuilder::apply_full_snapshot(&mut self.render_world, snap);
            self.timings.frame_build = Some(build_start.elapsed());
            let _ = (&self.frame_builder, &self.resources, &self.passes);
        }

        if !self.camera_ready {
            if let (Some(gpu), Some(sprite)) = (self.gpu.as_ref(), self.sprite.as_ref()) {
                let (iw, ih) = sprite.size();
                self.reset_camera_to_fit(gpu.config.width, gpu.config.height, iw, ih);
            }
        }

        let Some(gpu) = self.gpu.as_ref()
        else {
            return;
        };
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => return,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        if let Some(sprite) = self.sprite.as_ref() {
            sprite.write_vertices(&gpu.queue, &self.camera, gpu.config.width, gpu.config.height);
        }
        if let (Some(markers), Some(snap)) = (self.markers.as_mut(), snap) {
            markers.write_from_snapshot(&gpu.queue, snap, &self.camera, gpu.config.width, gpu.config.height);
        }
        else if let Some(markers) = self.markers.as_mut() {
            markers.clear();
        }

        let submit_start = std::time::Instant::now();
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ra.frame") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ra.frame_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(CLEAR_COLOR), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(sprite) = self.sprite.as_ref() {
                sprite.draw(&mut pass);
            }
            if let Some(markers) = self.markers.as_ref() {
                markers.draw(&mut pass);
            }
        }
        gpu.queue.submit(std::iter::once(encoder.finish()));
        self.timings.gpu_submit = Some(submit_start.elapsed());
        gpu.queue.present(frame);
    }

    /// 只读访问可复用渲染世界。
    pub fn render_world(&self) -> &RenderWorld {
        &self.render_world
    }

    /// 只读访问阶段图。
    pub fn pass_graph(&self) -> &PassGraph {
        &self.passes
    }

    /// 当前 wgpu 后端标签；未绑定时返回 `"wgpu(pending)"`。
    pub fn backend_name(&self) -> &'static str {
        self.gpu.as_ref().map(|g| g.backend_label()).unwrap_or("wgpu(pending)")
    }

    /// 是否已加载预览纹理（GPU 或待上传缓存均算有预览）。
    pub fn has_preview(&self) -> bool {
        self.sprite.is_some()
    }

    /// 按游戏版本返回渲染后端提示字符串（当前恒为 `"wgpu"`）。
    pub fn backend_hint(edition: GameEdition) -> &'static str {
        let _ = edition;
        "wgpu"
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
