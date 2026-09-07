//! 帧缓冲回读：截图验收用（非游戏内相册功能）。

use ra_types::{RaError, RaResult};

use crate::gpu::GpuContext;
use crate::rgba_image::RgbaImage;

/// 将表面纹理复制到 CPU 侧 RGBA8（行主序）。
///
/// `surface_texture` 须已完成本帧绘制且带 `COPY_SRC`。
pub fn readback_surface_rgba(
    gpu: &GpuContext,
    surface_texture: &wgpu::Texture,
) -> RaResult<RgbaImage> {
    let width = gpu.config.width.max(1);
    let height = gpu.config.height.max(1);
    let bpp = 4u32;
    let unpadded_bytes_per_row = width * bpp;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
    let buffer_size = padded_bytes_per_row as u64 * height as u64;

    let output = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ra.screenshot.buf"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ra.screenshot.copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: surface_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit(std::iter::once(encoder.finish()));

    let slice = output.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| RaError::Msg(format!("截图 poll 失败: {e}")))?;
    rx.recv()
        .map_err(|_| RaError::Msg("截图 map 通道断开".into()))?
        .map_err(|e| RaError::Msg(format!("截图 map 失败: {e}")))?;

    let data = slice
        .get_mapped_range()
        .map_err(|e| RaError::Msg(format!("截图 get_mapped_range 失败: {e}")))?;
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    let bgra = matches!(
        gpu.config.format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    for y in 0..height as usize {
        let src_row = y * padded_bytes_per_row as usize;
        let dst_row = y * width as usize * 4;
        for x in 0..width as usize {
            let s = src_row + x * 4;
            let d = dst_row + x * 4;
            if bgra {
                pixels[d] = data[s + 2];
                pixels[d + 1] = data[s + 1];
                pixels[d + 2] = data[s];
                pixels[d + 3] = data[s + 3];
            }
            else {
                pixels[d..d + 4].copy_from_slice(&data[s..s + 4]);
            }
        }
    }
    drop(data);
    output.unmap();

    RgbaImage::new(width, height, pixels).ok_or_else(|| RaError::Msg("截图像素尺寸非法".into()))
}
