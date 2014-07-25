//! wgpu 设备 / 表面初始化（桌面路径）。

use std::sync::Arc;

use pollster::FutureExt as _;
use ra_types::{RaError, RaResult};
use winit::window::Window;

pub struct GpuContext {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    backend: wgpu::Backend,
}

impl GpuContext {
    pub fn new(window: Arc<Window>) -> RaResult<Self> {
        Self::new_async(window).block_on()
    }

    async fn new_async(window: Arc<Window>) -> RaResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let surface = instance.create_surface(window.clone()).map_err(|e| RaError::Msg(format!("创建 wgpu 表面失败: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| RaError::Msg(format!("未找到可用 GPU: {e}")))?;

        let info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ra.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| RaError::Msg(format!("创建 wgpu 设备失败: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            // 优先不透明合成，避免部分驱动在首帧前透出桌面/白底。
            alpha_mode: caps
                .alpha_modes
                .iter()
                .copied()
                .find(|m| *m == wgpu::CompositeAlphaMode::Opaque)
                .unwrap_or(caps.alpha_modes[0]),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self { surface, device, queue, config, backend: info.backend })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.config.width == width && self.config.height == height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn backend_label(&self) -> &'static str {
        match self.backend {
            wgpu::Backend::Dx12 => "wgpu/dx12",
            wgpu::Backend::Vulkan => "wgpu/vulkan",
            wgpu::Backend::Metal => "wgpu/metal",
            wgpu::Backend::Gl => "wgpu/gl",
            wgpu::Backend::BrowserWebGpu => "wgpu/webgpu",
            _ => "wgpu",
        }
    }
}
