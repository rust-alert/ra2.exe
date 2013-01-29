//! 读取世界状态，经现代 GPU（wgpu）绘制。
//!
//! 原生后端：DX12 / Vulkan / Metal。Wasm：WebGL2。
//! 本 crate **故意不**实现 DirectDraw。

mod gpu;

use std::sync::Arc;

use ra_types::{GameEdition, RaResult};
use ra_world::World;
use winit::window::Window;

use crate::gpu::GpuContext;

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
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            frames: 0,
            gpu: None,
        }
    }

    /// 窗口就绪后绑定表面。可重复调用（忽略已绑定）。
    pub fn attach_window(&mut self, window: Arc<Window>) -> RaResult<()> {
        if self.gpu.is_some() {
            return Ok(());
        }
        self.gpu = Some(GpuContext::new(window)?);
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.resize(width, height);
        }
    }

    /// 清屏一帧。世界参数留给后续精灵/地形通道。
    pub fn draw_frame(&mut self, world: Option<&World>) {
        if let Some(world) = world {
            let _ = world.edition;
        }
        self.frames = self.frames.wrapping_add(1);
        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };
        let Ok(frame) = gpu.surface.get_current_texture() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ra.clear"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ra.clear_pass"),
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
