//! 集成测试：原 `src/sprite.rs` 内联测试迁出。

use ra_renderer::{SpriteColorSpace, target_format_for};

#[test]
fn encoded_bytes_target_strips_srgb_suffix() {
    assert_eq!(target_format_for(wgpu::TextureFormat::Bgra8UnormSrgb, SpriteColorSpace::EncodedBytes), wgpu::TextureFormat::Bgra8Unorm);
    assert_eq!(target_format_for(wgpu::TextureFormat::Rgba8UnormSrgb, SpriteColorSpace::EncodedBytes), wgpu::TextureFormat::Rgba8Unorm);
}

#[test]
fn srgb_preview_keeps_surface_format() {
    assert_eq!(target_format_for(wgpu::TextureFormat::Bgra8UnormSrgb, SpriteColorSpace::Srgb), wgpu::TextureFormat::Bgra8UnormSrgb);
}
