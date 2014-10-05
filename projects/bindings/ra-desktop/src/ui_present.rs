//! 壳层质感呈现：按 [`PresentFeel`] 把 8 位扩展色压回原版 16 位观感。
//!
//! 在合成 RGBA **上传 GPU 之前**调用。不改 SHP/调色板解码。
//!
//! 主路径是 RGB565/555 **截断量化 + 满量程线性展开**（对齐常见 16 位表面往返），
//! 不是用显示伽马去拧整体明暗。`gamma` / `highlight_roll_off` 仅作可选附加。

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
            // 抖动幅度对齐该通道丢弃的低位宽度（5bit→8，6bit→4）。
            let loss = 8u32 - u32::from(bits);
            let step = (1u32 << loss) as f32;
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

/// 16 位表面往返：截断到 `bits` 位，再按 `round(n * 255 / max)` 线性展开回 8 位。
///
/// 不用 bit-replicate（`(n<<k)|(n>>m)`）：那是另一种展开，和常见显示链扩表不一致。
fn quantize_channel(v: f32, bits: u8) -> u8 {
    let max_c = (1u32 << bits) - 1;
    let loss = 8u32 - u32::from(bits);
    let clamped = v.clamp(0.0, 255.0);
    let stepped = ((clamped as u32) >> loss).min(max_c);
    (((stepped * 255) + (max_c / 2)) / max_c) as u8
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
    fn default_uses_identity_gamma_and_rgb565_codebook() {
        let mut img = solid(200, 200, 200);
        let feel = PresentFeel {
            mode: PresentMode::Bit16,
            dither: false,
            ..PresentFeel::DEFAULT
        };
        assert!((feel.gamma - 1.0).abs() < 1e-6);
        assert!((feel.highlight_roll_off - 0.0).abs() < 1e-6);
        apply_present_feel(&mut img, feel);
        let v = img.as_raw()[0];
        // 200 >> 3 = 25 → round(25*255/31)=206。
        assert_eq!(v, 206);
    }

    #[test]
    fn five_bit_expand_is_linear_not_bit_replicate() {
        // 输入落在截断档 3：3<<3=24..31 → 取 24。
        assert_eq!(quantize_channel(24.0, 5), 25);
        // bit-replicate 会得到 24；线性满量程展开为 25。
        assert_ne!(quantize_channel(24.0, 5), (3u8 << 3) | (3u8 >> 2));
    }

    #[test]
    fn transparent_pixels_untouched() {
        let mut img = RgbaImage::from_raw(1, 1, vec![255, 255, 255, 0]).unwrap();
        apply_present_feel(&mut img, PresentFeel::DEFAULT);
        assert_eq!(img.as_raw(), &[255, 255, 255, 0]);
    }

    #[test]
    fn highlight_roll_off_crushes_near_white_more_than_mid_grey() {
        let feel = PresentFeel {
            mode: PresentMode::Bit16,
            dither: false,
            highlight_roll_off: 0.25,
            gamma: 1.0,
            ..PresentFeel::DEFAULT
        };
        let mut bright = solid(252, 252, 180);
        let mut mid = solid(120, 120, 120);
        apply_present_feel(&mut bright, feel);
        apply_present_feel(&mut mid, feel);
        let b = u16::from(bright.as_raw()[0]) + u16::from(bright.as_raw()[1]);
        let m = u16::from(mid.as_raw()[0]) + u16::from(mid.as_raw()[1]);
        // 近白应明显被压；中灰几乎不动（gamma=1）。
        assert!(b < 252 + 252 - 20, "near-white should roll off, got sum={b}");
        assert!(m >= 120 + 120 - 16, "mid grey should stay near input, got sum={m}");
    }
}
