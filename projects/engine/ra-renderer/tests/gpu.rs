//! 集成测试：原 `src/gpu.rs` 内联测试迁出。

use ra_renderer::encoded_view_formats;

#[test]
fn srgb_surface_exposes_unorm_view_format() {
    let views = encoded_view_formats(wgpu::TextureFormat::Bgra8UnormSrgb);
    assert_eq!(views, vec![wgpu::TextureFormat::Bgra8Unorm]);
}

#[test]
fn unorm_surface_needs_no_extra_view_format() {
    assert!(encoded_view_formats(wgpu::TextureFormat::Bgra8Unorm).is_empty());
}
