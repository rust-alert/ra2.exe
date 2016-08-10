//! present 质感变换集成测试。

use ra_renderer::RgbaImage;
use ra_types::{PresentFeel, PresentMode};
use ra_widgets::render::present::*;

fn solid(r: u8, g: u8, b: u8) -> RgbaImage {
    RgbaImage::from_raw(1, 1, vec![r, g, b, 255]).unwrap()
}

#[test]
fn off_is_noop() {
    let mut img = solid(200, 180, 160);
    apply_present_feel(&mut img, PresentFeel::OFF);
    assert_eq!(img.as_raw(), &[200, 180, 160, 255]);
}

#[test]
fn default_rgb565_codebook_without_gamma() {
    let mut img = solid(200, 200, 200);
    let feel = PresentFeel { mode: PresentMode::Bit16, dither: false, ..PresentFeel::DEFAULT };
    apply_present_feel(&mut img, feel);
    // 200 >> 3 = 25 → round(25*255/31)=206。
    assert_eq!(img.as_raw()[0], 206);
}

#[test]
fn five_bit_expand_is_linear_not_bit_replicate() {
    assert_eq!(quantize_channel_u8(24, 5), 25);
    assert_ne!(quantize_channel_u8(24, 5), (3u8 << 3) | (3u8 >> 2));
}

#[test]
fn transparent_pixels_untouched() {
    let mut img = RgbaImage::from_raw(1, 1, vec![255, 255, 255, 0]).unwrap();
    apply_present_feel(&mut img, PresentFeel::DEFAULT);
    assert_eq!(img.as_raw(), &[255, 255, 255, 0]);
}
