//! wgpu 设备 / 表面初始化（桌面窗口；Wasm 画布）。

use ra_types::{RaError, RaResult};

#[cfg(not(target_arch = "wasm32"))]
use pollster::FutureExt as _;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use winit::window::Window;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

/// GPU 上下文：surface + device + queue。
pub struct GpuContext {
    /// 交换链表面。
    pub surface: wgpu::Surface<'static>,
    /// 逻辑设备。
    pub device: wgpu::Device,
    /// 命令队列。
    pub queue: wgpu::Queue,
    /// 当前表面配置。
    pub config: wgpu::SurfaceConfiguration,
    backend: wgpu::Backend,
}

impl GpuContext {
    /// 桌面：从 winit 窗口创建（同步包装）。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(window: Arc<Window>) -> RaResult<Self> {
        Self::new_async(window).block_on()
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn new_async(window: Arc<Window>) -> RaResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).map_err(|e| RaError::Msg(format!("创建 wgpu 表面失败: {e}")))?;
        Self::finish(instance, surface, size.width.max(1), size.height.max(1), SurfaceUsageKind::Desktop).await
    }

    /// Wasm：从 HTML canvas 创建。
    #[cfg(target_arch = "wasm32")]
    pub async fn from_canvas(canvas: HtmlCanvasElement, width: u32, height: u32) -> RaResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface =
            instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).map_err(|e| RaError::Msg(format!("创建 wgpu 画布表面失败: {e}")))?;
        Self::finish(instance, surface, width.max(1), height.max(1), SurfaceUsageKind::Web).await
    }

    async fn finish(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        usage_kind: SurfaceUsageKind,
    ) -> RaResult<Self> {
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
        let required_limits = match usage_kind {
            #[cfg(not(target_arch = "wasm32"))]
            SurfaceUsageKind::Desktop => wgpu::Limits::default(),
            #[cfg(target_arch = "wasm32")]
            SurfaceUsageKind::Web => wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ra.device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| RaError::Msg(format!("创建 wgpu 设备失败: {e}")))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        // 壳层 UI 以「编码字节」写入时需 unorm 视图，避免把显示域字节当线性色再 sRGB 编码。
        let view_formats = encoded_view_formats(format);
        let usage = match usage_kind {
            #[cfg(not(target_arch = "wasm32"))]
            SurfaceUsageKind::Desktop => wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            // WebGL2 表面常不支持 COPY_SRC；截图路径桌面专用。
            #[cfg(target_arch = "wasm32")]
            SurfaceUsageKind::Web => wgpu::TextureUsages::RENDER_ATTACHMENT,
        };
        let config = wgpu::SurfaceConfiguration {
            usage,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            // 优先不透明合成，避免部分驱动在首帧前透出桌面/白底。
            alpha_mode: caps.alpha_modes.iter().copied().find(|m| *m == wgpu::CompositeAlphaMode::Opaque).unwrap_or(caps.alpha_modes[0]),
            view_formats,
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self { surface, device, queue, config, backend: info.backend })
    }

    /// 调整交换链尺寸。
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

    /// 后端标签（诊断）。
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

enum SurfaceUsageKind {
    #[cfg(not(target_arch = "wasm32"))]
    Desktop,
    #[cfg(target_arch = "wasm32")]
    Web,
}

/// 若 `format` 为 sRGB，则附加其 unorm 别名供编码域壳层绘制。
/// 若表面为 sRGB，则附加对应 unorm 视图格式。
pub fn encoded_view_formats(format: wgpu::TextureFormat) -> Vec<wgpu::TextureFormat> {
    let encoded = format.remove_srgb_suffix();
    if encoded != format { vec![encoded] } else { Vec::new() }
}
