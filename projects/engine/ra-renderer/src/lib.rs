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
mod capture;
mod frame;
mod gpu;
mod markers;
mod order_icons;
mod pass;
mod png_out;
mod resources;
mod sprite;
mod timings;
mod world;

use ra_engine::RenderSnapshot;
use ra_types::{GameEdition, RaResult};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use winit::window::Window;
#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

use crate::{camera::Camera, gpu::GpuContext, markers::MarkerGpu, order_icons::OrderIconGpu, sprite::SpriteGpu};

/// 2D 视口相机：平移与缩放，供外部读取或调整视角。
pub use crate::camera::Camera as ViewCamera;
/// 相机中心夹紧边界。
pub use crate::camera::CameraBounds;
/// 帧构建器（投影 → `RenderWorld`）。
pub use crate::frame::FrameBuilder;
/// 表面 sRGB → unorm 视图格式列表。
pub use crate::gpu::encoded_view_formats;
/// NDC 粗裁剪（marker stub）。
pub use crate::markers::ndc_visible;
/// 命令图标 CPU 解码结果（`mouse.shp` 帧）。
pub use crate::order_icons::DecodedOrderIcons;
/// 渲染阶段图。
pub use crate::pass::{PassGraph, RenderPassKind};
/// RGBA → PNG 字节。
pub use crate::png_out::encode_png;
/// 将 RGBA 写成 PNG 文件。
pub use crate::png_out::write_png_file;
/// GPU 资源缓存骨架。
pub use crate::resources::RenderResourceCache;
/// 精灵色域（壳层 UI 用编码字节直通）。
pub use crate::sprite::SpriteColorSpace;
/// 精灵 pipeline 目标格式。
pub use crate::sprite::target_format_for;
/// 帧分段计时。
pub use crate::timings::FrameTimings;
/// 可复用渲染世界。
pub use crate::world::RenderWorld;
/// `image` 的 RGBA 像素类型（便于后续 modder 管线复用）。
pub use image::Rgba;
/// CPU 侧 RGBA 像素缓冲（`image` crate），可上传到 GPU 作为预览纹理。
pub use image::RgbaImage;

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
    /// 原版壳层 UI 页（与地图预览分通道；CPU 合成图的过渡上传）。
    ui_page: Option<RgbaImage>,
    ui_sprite: Option<SpriteGpu>,
    /// 为真时 UI 为对局屏空间叠加层：不劫持世界相机，可与 preview/markers 同帧。
    ui_overlay: bool,
    /// 世界 pass 的投影 / scissor 矩形（窗口像素）。`None` 表示整窗表面。
    ///
    /// 对局叠加时应为战术区（侧栏以左），与命中、`CameraBounds` 同口径。
    world_view: Option<(u32, u32, u32, u32)>,
    markers: Option<MarkerGpu>,
    /// `mouse.shp` 命令图标图集（移动 / 攻击 / 部署）。
    order_icons: Option<OrderIconGpu>,
    /// GPU 未就绪时暂存的命令图标。
    pending_order_icons: Option<DecodedOrderIcons>,
    /// 下一帧 `submit_frame` 结束后做表面回读。
    capture_pending: bool,
    /// 最近一次成功截图（RGBA）。
    last_capture: Option<RgbaImage>,
    /// 最近一次回读失败说明（有值表示请求已结束但失败，不是「尚未回读」）。
    capture_error: Option<String>,
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
            ui_page: None,
            ui_sprite: None,
            ui_overlay: false,
            world_view: None,
            markers: None,
            order_icons: None,
            pending_order_icons: None,
            capture_pending: false,
            last_capture: None,
            capture_error: None,
            camera: Camera { center_x: 0.0, center_y: 0.0, zoom: 1.0 },
            camera_ready: false,
            render_world: RenderWorld::default(),
            resources: RenderResourceCache::default(),
            passes: PassGraph::prototype_default(),
            frame_builder: FrameBuilder,
        }
    }

    /// 请求在下一帧提交后回读表面（用于关键页验收截图）。
    pub fn request_capture(&mut self) {
        self.capture_pending = true;
        self.capture_error = None;
    }

    /// 取走最近一次成功回读结果（若有）。
    pub fn take_capture(&mut self) -> Option<RgbaImage> {
        self.last_capture.take()
    }

    /// 取走最近一次回读失败说明（若有）。与「尚无结果」区分开。
    pub fn take_capture_error(&mut self) -> Option<String> {
        self.capture_error.take()
    }

    /// 设置启动预览图（窗口附着后上传）。
    ///
    /// 适用于地图缩略图等**内容预览**，不是原版菜单/HUD 通道。
    pub fn set_preview(&mut self, image: RgbaImage) {
        self.set_map_preview(image);
    }

    /// 设置地图 / 地形预览图（与未来 UI 页通道分离的命名入口）。
    pub fn set_map_preview(&mut self, image: RgbaImage) {
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
            self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width(), image.height());
        }
        else {
            self.camera_ready = false;
        }
        self.preview = Some(image);
    }

    /// 更新地图预览像素，**不**重置相机（活动层逐帧刷新用）。
    ///
    /// 尺寸变化时回退为 [`Self::set_map_preview`]。
    pub fn update_map_preview(&mut self, image: RgbaImage) {
        let size_changed = self.preview.as_ref().is_none_or(|p| p.width() != image.width() || p.height() != image.height());
        if size_changed || self.sprite.is_none() {
            self.set_map_preview(image);
            return;
        }
        if let Some(gpu) = self.gpu.as_ref() {
            if let Some(sprite) = self.sprite.as_mut() {
                sprite.replace_image(&gpu.device, &gpu.queue, &image);
            }
        }
        self.preview = Some(image);
    }

    /// 清空预览底图（CPU 缓存与 GPU sprite）。
    ///
    /// 前置菜单未接原版 UI 时用于诚实空屏，避免残留过期缩略图。
    pub fn clear_preview(&mut self) {
        self.preview = None;
        self.sprite = None;
        if self.ui_page.is_none() {
            self.camera_ready = false;
        }
    }

    /// 设置原版壳层 UI 页纹理（与 [`Self::set_map_preview`] 分通道）。
    ///
    /// 当前接受已合成的整页 RGBA，作为 atlas/instance UI pass 之前的过渡上传路径。
    /// 使用 [`SpriteColorSpace::EncodedBytes`]：CPU 侧已是显示域字节，GPU 不再按 sRGB 线性化。
    /// 菜单全页模式：相机会 letterbox 到该页。对局叠加请用 [`Self::set_ui_overlay`]。
    pub fn set_ui_page(&mut self, image: RgbaImage) {
        self.ui_overlay = false;
        self.world_view = None;
        self.upload_ui_texture(image, true);
    }

    /// 设置对局 HUD 叠加层：不重置世界相机，与地图预览 / markers 同帧绘制。
    pub fn set_ui_overlay(&mut self, image: RgbaImage) {
        self.ui_overlay = true;
        self.upload_ui_texture(image, false);
    }

    /// 设置世界 pass 投影与裁切矩形（窗口像素，`x,y,w,h`）。
    ///
    /// 对局热路径应与 `MapViewport::clip_rect_u32` 一致。宽或高为 0 时清除。
    pub fn set_world_view_rect(&mut self, x: u32, y: u32, w: u32, h: u32) {
        if w == 0 || h == 0 {
            self.world_view = None;
            return;
        }
        self.world_view = Some((x, y, w, h));
        let (pw, ph) = self.world_proj_size();
        if let Some(bounds) = self.camera_bounds_for_viewport(pw, ph) {
            self.camera.clamp_to_bounds(&bounds);
        }
    }

    /// 清除世界 pass 专用视口，恢复整窗投影。
    pub fn clear_world_view_rect(&mut self) {
        self.world_view = None;
        let (pw, ph) = self.world_proj_size();
        if pw > 0.0 && ph > 0.0 {
            if let Some(bounds) = self.camera_bounds_for_viewport(pw, ph) {
                self.camera.clamp_to_bounds(&bounds);
            }
        }
    }

    /// 当前世界投影矩形；未设置时为整窗表面。
    pub fn world_view_rect(&self) -> Option<(u32, u32, u32, u32)> {
        self.world_view
    }

    fn upload_ui_texture(&mut self, image: RgbaImage, fit_camera: bool) {
        if let Some(gpu) = self.gpu.as_ref() {
            match self.ui_sprite.as_mut() {
                Some(sprite) => sprite.replace_image(&gpu.device, &gpu.queue, &image),
                None => {
                    self.ui_sprite = Some(SpriteGpu::create_with_color_space(
                        &gpu.device,
                        &gpu.queue,
                        gpu.config.format,
                        &image,
                        crate::sprite::SpriteColorSpace::EncodedBytes,
                    ));
                }
            }
            if fit_camera {
                self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width(), image.height());
            }
        }
        else if fit_camera {
            self.camera_ready = false;
        }
        self.ui_page = Some(image);
    }

    /// 清空壳层 UI 页。
    pub fn clear_ui_page(&mut self) {
        self.ui_page = None;
        self.ui_sprite = None;
        self.ui_overlay = false;
        self.world_view = None;
        if self.preview.is_none() {
            self.camera_ready = false;
        }
    }

    /// 是否已有壳层 UI 页。
    pub fn has_ui_page(&self) -> bool {
        self.ui_page.is_some()
    }

    /// 窗口就绪后绑定表面。可重复调用（忽略已绑定）。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn attach_window(&mut self, window: Arc<Window>) -> RaResult<()> {
        if self.gpu.is_some() {
            return Ok(());
        }
        let gpu = GpuContext::new(window)?;
        self.bind_gpu(gpu);
        Ok(())
    }

    /// 画布就绪后绑定表面（Wasm）。可重复调用（忽略已绑定）。
    #[cfg(target_arch = "wasm32")]
    pub async fn attach_canvas(&mut self, canvas: HtmlCanvasElement) -> RaResult<()> {
        if self.gpu.is_some() {
            return Ok(());
        }
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let gpu = GpuContext::from_canvas(canvas, width, height).await?;
        self.bind_gpu(gpu);
        Ok(())
    }

    fn bind_gpu(&mut self, gpu: GpuContext) {
        if let Some(image) = self.ui_page.as_ref() {
            self.ui_sprite = Some(SpriteGpu::create_with_color_space(
                &gpu.device,
                &gpu.queue,
                gpu.config.format,
                image,
                crate::sprite::SpriteColorSpace::EncodedBytes,
            ));
            self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width(), image.height());
        }
        else if let Some(image) = self.preview.as_ref() {
            self.sprite = Some(SpriteGpu::create(&gpu.device, &gpu.queue, gpu.config.format, image));
            self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width(), image.height());
        }
        self.markers = Some(MarkerGpu::create(&gpu.device, gpu.config.format));
        if let Some(icons) = self.pending_order_icons.take() {
            self.order_icons = OrderIconGpu::create(&gpu.device, &gpu.queue, gpu.config.format, &icons);
        }
        self.gpu = Some(gpu);
    }

    /// 装入 `mouse.shp` 命令图标（移动 / 攻击 / 部署）。GPU 未就绪时暂存。
    pub fn set_order_icons(&mut self, icons: DecodedOrderIcons) {
        if let Some(gpu) = self.gpu.as_ref() {
            self.order_icons = OrderIconGpu::create(&gpu.device, &gpu.queue, gpu.config.format, &icons);
            self.pending_order_icons = None;
        }
        else {
            self.pending_order_icons = Some(icons);
            self.order_icons = None;
        }
    }

    /// 当前 GPU 后端标签（未附着时为 `None`）。
    pub fn backend_label(&self) -> Option<&'static str> {
        self.gpu.as_ref().map(GpuContext::backend_label)
    }

    /// 通知交换链表面尺寸变化（像素宽高）。
    ///
    /// 相机已就绪时只按新视口重夹边界（保留中心与缩放，避免对局开局对焦后再被整图 fit 冲掉）。
    /// 尚未就绪时才按当前活动底图 letterbox fit。
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.resize(width, height);
        }
        let (sw, sh) = self.gpu.as_ref().map(|g| (g.config.width, g.config.height)).unwrap_or((width.max(1), height.max(1)));
        if self.camera_ready {
            let (pw, ph) = self.world_proj_size();
            let (bw, bh) = if pw > 0.0 && ph > 0.0 { (pw, ph) } else { (sw as f32, sh as f32) };
            if let Some(bounds) = self.camera_bounds_for_viewport(bw, bh) {
                self.camera.clamp_to_bounds(&bounds);
            }
            return;
        }
        if let Some(ui) = self.ui_sprite.as_ref() {
            let (iw, ih) = ui.size();
            self.reset_camera_to_fit(sw, sh, iw, ih);
        }
        else if let Some(sprite) = self.sprite.as_ref() {
            let (iw, ih) = sprite.size();
            self.reset_camera_to_fit(sw, sh, iw, ih);
        }
    }

    /// 将视口对准世界坐标（预览图像素），并设为指定缩放（会夹到地图边界）。
    pub fn focus_camera(&mut self, world_x: f32, world_y: f32, zoom: f32) {
        self.camera.zoom = zoom.clamp(Camera::ZOOM_MIN, Camera::ZOOM_MAX);
        self.camera.center_x = world_x;
        self.camera.center_y = world_y;
        let (vw, vh) = self.world_proj_size();
        if vw > 0.0 && vh > 0.0 {
            if let Some(bounds) = self.camera_bounds_for_viewport(vw, vh) {
                self.camera.clamp_to_bounds(&bounds);
            }
        }
        self.camera_ready = true;
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

    /// 平移视口并夹到当前地图预览边界（小图居中，大图不可拖出黑边）。
    pub fn pan_clamped(&mut self, dx: f32, dy: f32) {
        let (vw, vh) = self.world_proj_size();
        self.pan_clamped_in_viewport(dx, dy, vw, vh);
    }

    /// 在指定可视矩形（屏幕像素）内平移并夹紧。
    ///
    /// `viewport_w` / `viewport_h` 必须与当前帧投影用的屏尺寸一致。对局热路径为
    /// 战术区宽高（与 [`Self::set_world_view_rect`] / `write_vertices` 同口径）。
    /// 传入比投影更小的矩形会使 `CameraBounds` 过松，拖出预览外的 void。
    pub fn pan_clamped_in_viewport(&mut self, dx: f32, dy: f32, viewport_w: f32, viewport_h: f32) {
        let Some(bounds) = self.camera_bounds_for_viewport(viewport_w, viewport_h)
        else {
            self.camera.pan_screen(dx, dy);
            return;
        };
        self.camera.pan_clamped(dx, dy, &bounds);
    }

    /// 相对缩放视口，`factor` 大于 1 为放大。
    pub fn zoom_by(&mut self, factor: f32) {
        self.camera.zoom_by(factor);
        let (vw, vh) = self.world_proj_size();
        if let Some(bounds) = self.camera_bounds_for_viewport(vw, vh) {
            self.camera.clamp_to_bounds(&bounds);
        }
    }

    /// 当前预览世界与给定 viewport 下的相机边界；无预览时为 `None`。
    pub fn camera_bounds_for_viewport(&self, viewport_w: f32, viewport_h: f32) -> Option<CameraBounds> {
        let preview = self.preview.as_ref()?;
        Some(crate::camera::CameraBounds::from_world_and_viewport(
            preview.width() as f32,
            preview.height() as f32,
            viewport_w.max(1.0),
            viewport_h.max(1.0),
            self.camera.zoom,
        ))
    }

    /// 当前预览世界与表面尺寸下的相机边界；无预览或未绑定 GPU 时为 `None`。
    pub fn camera_bounds(&self) -> Option<CameraBounds> {
        let (vw, vh) = self.world_proj_size();
        if vw <= 0.0 || vh <= 0.0 {
            return None;
        }
        self.camera_bounds_for_viewport(vw, vh)
    }

    fn surface_size(&self) -> (f32, f32) {
        let Some(gpu) = self.gpu.as_ref()
        else {
            return (0.0, 0.0);
        };
        (gpu.config.width.max(1) as f32, gpu.config.height.max(1) as f32)
    }

    /// 当前交换链表面尺寸（像素）；未绑定 GPU 时为 `None`。
    pub fn surface_size_u32(&self) -> Option<(u32, u32)> {
        self.gpu.as_ref().map(|g| (g.config.width.max(1), g.config.height.max(1)))
    }

    /// 世界投影用的宽高：已设 `world_view` 时用战术区，否则整窗。
    fn world_proj_size(&self) -> (f32, f32) {
        if let Some((_, _, w, h)) = self.world_view {
            return (w.max(1) as f32, h.max(1) as f32);
        }
        self.surface_size()
    }

    fn reset_camera_to_fit(&mut self, screen_w: u32, screen_h: u32, image_w: u32, image_h: u32) {
        self.camera = Camera::fit(image_w, image_h, screen_w, screen_h);
        self.camera_ready = true;
    }

    /// 清屏并提交（可选全量同步 `RenderWorld`）。
    ///
    /// 对局热路径请优先 [`Self::draw_incremental`]。菜单等无会话场景传 `None`。
    /// 保留调用方已写入的 `timings.simulation` / `presentation_build`。
    pub fn draw_frame(&mut self, snap: Option<&RenderSnapshot>) {
        self.begin_frame_timings();
        if let Some(snap) = snap {
            let build_start = std::time::Instant::now();
            FrameBuilder::apply_full_snapshot(&mut self.render_world, snap);
            self.timings.frame_build = Some(build_start.elapsed());
            let _ = (&self.frame_builder, &self.resources, &self.passes);
        }
        else {
            self.render_world.clear_units();
        }
        self.submit_frame();
    }

    /// 用脏实体投影增量更新 `RenderWorld` 并提交（不重建整表）。
    pub fn draw_incremental(
        &mut self,
        source_tick: u64,
        dirty: &[ra_types::EntityId],
        units: &[ra_engine::SnapshotUnit],
        selected: &[ra_types::EntityId],
    ) {
        self.begin_frame_timings();
        let build_start = std::time::Instant::now();
        FrameBuilder::apply_dirty_units(&mut self.render_world, source_tick, dirty, units, selected);
        self.timings.frame_build = Some(build_start.elapsed());
        let _ = (&self.frame_builder, &self.resources, &self.passes);
        self.submit_frame();
    }

    /// 重开对局前清空可视槽，迫使下一帧全量同步。
    pub fn clear_match_visuals(&mut self) {
        self.render_world.clear_units();
        self.render_world.action_lines_active = false;
    }

    /// 本帧是否绘制选中行动线（UnitActionLines 窗口）。
    pub fn set_action_lines_active(&mut self, active: bool) {
        self.render_world.action_lines_active = active;
    }

    fn begin_frame_timings(&mut self) {
        self.frames = self.frames.wrapping_add(1);
        let keep_sim = self.timings.simulation;
        let keep_pres = self.timings.presentation_build;
        self.timings.clear();
        self.timings.simulation = keep_sim;
        self.timings.presentation_build = keep_pres;
    }

    fn submit_frame(&mut self) {
        if !self.camera_ready {
            if let Some(gpu) = self.gpu.as_ref() {
                if self.ui_overlay {
                    if let Some(sprite) = self.sprite.as_ref() {
                        let (iw, ih) = sprite.size();
                        self.reset_camera_to_fit(gpu.config.width, gpu.config.height, iw, ih);
                    }
                }
                else if let Some(ui) = self.ui_sprite.as_ref() {
                    let (iw, ih) = ui.size();
                    self.reset_camera_to_fit(gpu.config.width, gpu.config.height, iw, ih);
                }
                else if let Some(sprite) = self.sprite.as_ref() {
                    let (iw, ih) = sprite.size();
                    self.reset_camera_to_fit(gpu.config.width, gpu.config.height, iw, ih);
                }
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

        let overlay = self.ui_overlay && self.ui_sprite.is_some();
        let world_view = self.world_view;
        let world_proj = match world_view {
            Some((_, _, w, h)) if overlay => (w.max(1), h.max(1)),
            _ => (gpu.config.width.max(1), gpu.config.height.max(1)),
        };
        let world_sprite = if overlay { self.sprite.as_ref() } else { self.ui_sprite.as_ref().or(self.sprite.as_ref()) };
        let encoded_menu_ui = !overlay && self.ui_sprite.as_ref().is_some_and(SpriteGpu::is_encoded_bytes);
        let srgb_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let unorm_view = self.ui_sprite.as_ref().filter(|s| s.is_encoded_bytes()).map(|ui| {
            frame.texture.create_view(&wgpu::TextureViewDescriptor { format: Some(ui.target_format()), ..Default::default() })
        });

        if let Some(sprite) = world_sprite {
            sprite.write_vertices(&gpu.queue, &self.camera, world_proj.0, world_proj.1);
        }
        let ui_cam = if overlay {
            self.ui_sprite.as_ref().map(|ui| {
                let (iw, ih) = ui.size();
                Camera::fit(iw, ih, gpu.config.width, gpu.config.height)
            })
        }
        else {
            None
        };
        if let (Some(ui), Some(cam)) = (self.ui_sprite.as_ref(), ui_cam.as_ref()) {
            ui.write_vertices(&gpu.queue, cam, gpu.config.width, gpu.config.height);
        }
        else if !overlay {
            if let Some(ui) = self.ui_sprite.as_ref() {
                ui.write_vertices(&gpu.queue, &self.camera, gpu.config.width, gpu.config.height);
            }
        }

        if let Some(markers) = self.markers.as_mut() {
            let draw_markers = self.render_world.unit_count() > 0 && (overlay || self.ui_sprite.is_none());
            if draw_markers {
                markers.write_from_world(
                    &gpu.queue,
                    &self.render_world,
                    &self.camera,
                    world_proj.0,
                    world_proj.1,
                );
            }
            else {
                markers.clear();
            }
        }
        if let Some(icons) = self.order_icons.as_mut() {
            let draw_icons = self.render_world.unit_count() > 0 && (overlay || self.ui_sprite.is_none());
            if draw_icons {
                icons.write_from_world(
                    &gpu.queue,
                    &self.render_world,
                    &self.camera,
                    world_proj.0,
                    world_proj.1,
                );
            }
            else {
                icons.clear();
            }
        }

        let clear = if encoded_menu_ui { wgpu::Color::BLACK } else { CLEAR_COLOR };
        let submit_start = std::time::Instant::now();
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ra.frame") });

        if overlay {
            // Pass 1：世界（预览 + markers），sRGB 视图；裁切到战术区。
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ra.frame_pass.world"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &srgb_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(CLEAR_COLOR), store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if let Some((vx, vy, vw, vh)) = world_view {
                    let vw = vw.max(1);
                    let vh = vh.max(1);
                    pass.set_viewport(vx as f32, vy as f32, vw as f32, vh as f32, 0.0, 1.0);
                    pass.set_scissor_rect(vx, vy, vw, vh);
                }
                if let Some(sprite) = self.sprite.as_ref() {
                    sprite.draw(&mut pass);
                }
                if let Some(markers) = self.markers.as_ref() {
                    if self.render_world.unit_count() > 0 {
                        markers.draw(&mut pass);
                    }
                }
                if let Some(icons) = self.order_icons.as_ref() {
                    if self.render_world.unit_count() > 0 {
                        icons.draw(&mut pass);
                    }
                }
            }
            // Pass 2：HUD 叠加（编码域 unorm），保留世界内容。
            if let (Some(ui), Some(view)) = (self.ui_sprite.as_ref(), unorm_view.as_ref()) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ra.frame_pass.hud_overlay"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                ui.draw(&mut pass);
            }
        }
        else {
            let view = unorm_view.as_ref().unwrap_or(&srgb_view);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(if encoded_menu_ui { "ra.frame_pass.encoded_ui" } else { "ra.frame_pass" }),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some(sprite) = world_sprite {
                sprite.draw(&mut pass);
            }
            if let Some(markers) = self.markers.as_ref() {
                if self.ui_sprite.is_none() && self.render_world.unit_count() > 0 {
                    markers.draw(&mut pass);
                }
            }
            if let Some(icons) = self.order_icons.as_ref() {
                if self.ui_sprite.is_none() && self.render_world.unit_count() > 0 {
                    icons.draw(&mut pass);
                }
            }
        }

        gpu.queue.submit(std::iter::once(encoder.finish()));
        self.timings.gpu_submit = Some(submit_start.elapsed());

        if self.capture_pending {
            self.capture_pending = false;
            match crate::capture::readback_surface_rgba(gpu, &frame.texture) {
                Ok(img) => {
                    self.capture_error = None;
                    self.last_capture = Some(img);
                }
                Err(e) => {
                    self.last_capture = None;
                    self.capture_error = Some(e.to_string());
                }
            }
        }

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
