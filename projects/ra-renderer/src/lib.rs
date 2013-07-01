//! 读取呈现快照，经现代 GPU（wgpu）绘制。
//!
//! 原生后端：DX12 / Vulkan / Metal。Wasm：WebGL2。
//! 本 crate **故意不**实现 DirectDraw。

mod camera;
mod gpu;
mod markers;
mod rgba_image;
mod sprite;

use std::sync::Arc;

use ra_session::RenderSnapshot;
use ra_types::{GameEdition, RaResult};
use winit::window::Window;

use crate::camera::Camera;
use crate::gpu::GpuContext;
use crate::markers::MarkerGpu;
use crate::sprite::SpriteGpu;

pub use crate::camera::Camera as ViewCamera;
pub use crate::rgba_image::RgbaImage;

/// 清屏底色（接近夜间战术图感觉，非最终主题）。
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.04,
    g: 0.06,
    b: 0.09,
    a: 1.0,
};

pub struct Renderer {
    pub frames: u64,
    gpu: Option<GpuContext>,
    preview: Option<RgbaImage>,
    sprite: Option<SpriteGpu>,
    markers: Option<MarkerGpu>,
    camera: Camera,
    camera_ready: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            frames: 0,
            gpu: None,
            preview: None,
            sprite: None,
            markers: None,
            camera: Camera {
                center_x: 0.0,
                center_y: 0.0,
                zoom: 1.0,
            },
            camera_ready: false,
        }
    }

    /// 设置启动预览图（窗口附着后上传）。
    pub fn set_preview(&mut self, image: RgbaImage) {
        if let Some(gpu) = self.gpu.as_ref() {
            match self.sprite.as_mut() {
                Some(sprite) => sprite.replace_image(&gpu.device, &gpu.queue, &image),
                None => {
                    self.sprite = Some(SpriteGpu::create(
                        &gpu.device,
                        &gpu.queue,
                        gpu.config.format,
                        &image,
                    ));
                }
            }
            if self.markers.is_none() {
                self.markers = Some(MarkerGpu::create(&gpu.device, gpu.config.format));
            }
            self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width, image.height);
        } else {
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
            self.sprite = Some(SpriteGpu::create(
                &gpu.device,
                &gpu.queue,
                gpu.config.format,
                image,
            ));
            self.reset_camera_to_fit(gpu.config.width, gpu.config.height, image.width, image.height);
        }
        self.markers = Some(MarkerGpu::create(&gpu.device, gpu.config.format));
        self.gpu = Some(gpu);
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.resize(width, height);
        }
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn pan_screen(&mut self, dx: f32, dy: f32) {
        self.camera.pan_screen(dx, dy);
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.camera.zoom_by(factor);
    }

    fn reset_camera_to_fit(&mut self, screen_w: u32, screen_h: u32, image_w: u32, image_h: u32) {
        self.camera = Camera::fit(image_w, image_h, screen_w, screen_h);
        self.camera_ready = true;
    }

    /// 清屏：预览底图 + 快照单位标记。
    pub fn draw_frame(&mut self, snap: Option<&RenderSnapshot>) {
        self.frames = self.frames.wrapping_add(1);

        if !self.camera_ready {
            if let (Some(gpu), Some(sprite)) = (self.gpu.as_ref(), self.sprite.as_ref()) {
                let (iw, ih) = sprite.size();
                self.reset_camera_to_fit(gpu.config.width, gpu.config.height, iw, ih);
            }
        }

        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };
        let Ok(frame) = gpu.surface.get_current_texture() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        if let Some(sprite) = self.sprite.as_ref() {
            sprite.write_vertices(
                &gpu.queue,
                &self.camera,
                gpu.config.width,
                gpu.config.height,
            );
        }
        if let (Some(markers), Some(snap)) = (self.markers.as_mut(), snap) {
            markers.write_from_snapshot(
                &gpu.queue,
                snap,
                &self.camera,
                gpu.config.width,
                gpu.config.height,
            );
        } else if let Some(markers) = self.markers.as_mut() {
            markers.clear();
        }

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ra.frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ra.frame_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(sprite) = self.sprite.as_ref() {
                sprite.draw(&mut pass);
            }
            if let Some(markers) = self.markers.as_ref() {
                markers.draw(&mut pass);
            }
        }
        gpu.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn backend_name(&self) -> &'static str {
        self.gpu
            .as_ref()
            .map(|g| g.backend_label())
            .unwrap_or("wgpu(pending)")
    }

    pub fn has_preview(&self) -> bool {
        self.sprite.is_some()
    }

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
