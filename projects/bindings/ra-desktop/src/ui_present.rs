//! 壳层质感呈现：按 [`PresentFeel`] 把 8 位扩展色压回原版 16 位观感。
//!
//! 在合成 RGBA **上传 GPU 之前**调用。不改 SHP/调色板解码。

use ra_renderer::RgbaImage;
use ra_types::{PresentFeel, PresentQuantize};

/// 4×4 Bayer 有序抖动矩阵（0..15）。
const BAYER4: [[u8; 4]; 4] = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [13, 7, 15, 5],
];

/// 就地应用质感呈现；`mode = off` 时无操作。
pub fn apply_present_feel(image: &mut RgbaImage, feel: PresentFeel) {
    let feel = feel.sanitized();
    if !feel.is_active() {
        return;
    }

    let gamma_lut = build_gamma_lut(feel.gamma);
    let roll = feel.highlight_roll_off;
    let dither_on = feel.dither;
    let quant = feel.quantize;
    let w = image.width();
    let raw = image.as_mut();

    for (i, px) in raw.chunks_exact_mut(4).enumerate() {
        if px[3] == 0 {
            continue;
        }
        let x = (i as u32) % w;
        let y = (i as u32) / w;
        let dither = if dither_on {
            let b = f32::from(BAYER4[(y as usize) & 3][(x as usize) & 3]);
            (b / 16.0) - 0.5
        } else {
            0.0
        };

        for c in 0..3 {
            let mut v = f32::from(gamma_lut[px[c] as usize]);
            if roll > 0.0 {
                let t = ((v / 255.0) - 0.70).clamp(0.0, 1.0) / 0.30;
                v *= 1.0 - roll * t * t;
            }
            let bits = match (quant, c) {
                (PresentQuantize::Rgb565, 1) => 6,
                (PresentQuantize::Rgb565, _) => 5,
                (PresentQuantize::Rgb555, _) => 5,
            };
            let step = 255.0 / ((1u32 << bits) as f32 - 1.0);
            px[c] = quantize_channel(v + dither * step, bits);
        }
    }
}

/// 消费图像并返回呈现后的副本（`off` 时原样返回）。
pub fn present_ui_page(mut image: RgbaImage, feel: PresentFeel) -> RgbaImage {
    apply_present_feel(&mut image, feel);
    image
}

fn build_gamma_lut(gamma: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    let g = if gamma.is_finite() && gamma > 0.0 { gamma } else { 1.0 };
    for i in 0..256 {
        let x = (i as f32) / 255.0;
        let y = x.powf(g).clamp(0.0, 1.0);
        lut[i] = (y * 255.0 + 0.5) as u8;
    }
    lut
}

fn quantize_channel(v: f32, bits: u8) -> u8 {
    let max_c = ((1u32 << bits) - 1) as f32;
    let stepped = ((v.clamp(0.0, 255.0) / 255.0) * max_c).round().clamp(0.0, max_c) as u8;
    match bits {
        5 => (stepped << 3) | (stepped >> 2),
        6 => (stepped << 2) | (stepped >> 4),
        _ => stepped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_types::PresentMode;

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
    fn bit16_darkens_mid_highlights() {
        let mut img = solid(200, 200, 200);
        let feel = PresentFeel {
            mode: PresentMode::Bit16,
            dither: false,
            ..PresentFeel::DEFAULT
        };
        apply_present_feel(&mut img, feel);
        let v = img.as_raw()[0];
        // 默认 gamma≈1.15：200 → 约 178（对齐侧栏金属亮度，勿压到过暗）。
        assert!(v < 195, "gamma should darken mid-bright grey, got {v}");
        assert!(v > 160, "should not crush too hard, got {v}");
    }

    #[test]
    fn transparent_pixels_untouched() {
        let mut img = RgbaImage::from_raw(1, 1, vec![255, 255, 255, 0]).unwrap();
        apply_present_feel(&mut img, PresentFeel::DEFAULT);
        assert_eq!(img.as_raw(), &[255, 255, 255, 0]);
    }
}
