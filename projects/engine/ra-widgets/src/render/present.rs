//! 壳层质感呈现：按 [`PresentFeel`] 把 8 位扩展色压回原版 16 位观感。
//!
//! 在合成 RGBA **上传 GPU 之前**调用。不改 SHP/调色板解码。
//!
//! 只做 RGB565/555 **截断量化 + 满量程线性展开**（可选有序抖动）。
//! GPU 侧壳层页以编码字节直通交换链 unorm 视图。

use ra_renderer::RgbaImage;
use ra_types::{PresentFeel, PresentQuantize};

/// 4×4 Bayer 有序抖动矩阵（0..15）。
const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [13, 7, 15, 5]];

/// 就地应用质感呈现；`mode = off` 时无操作。
pub fn apply_present_feel(image: &mut RgbaImage, feel: PresentFeel) {
    let feel = feel.sanitized();
    if !feel.is_active() {
        return;
    }

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
        }
        else {
            0.0
        };

        for c in 0..3 {
            let bits = match (quant, c) {
                (PresentQuantize::Rgb565, 1) => 6,
                (PresentQuantize::Rgb565, _) => 5,
                (PresentQuantize::Rgb555, _) => 5,
            };
            if dither_on {
                let loss = 8u32 - u32::from(bits);
                let step = (1u32 << loss) as f32;
                px[c] = quantize_channel_f32(f32::from(px[c]) + dither * step, bits);
            }
            else {
                px[c] = quantize_channel_u8(px[c], bits);
            }
        }
    }
}

/// 消费图像并返回呈现后的副本（`off` 时原样返回）。
pub fn present_ui_page(mut image: RgbaImage, feel: PresentFeel) -> RgbaImage {
    apply_present_feel(&mut image, feel);
    image
}

/// 16 位表面往返：截断到 `bits` 位，再按 `round(n * 255 / max)` 线性展开回 8 位。
/// 截断到 \its\ 位再线性展开回 8 位。
/// 截断到 `bits` 位再线性展开回 8 位。
pub fn quantize_channel_u8(v: u8, bits: u8) -> u8 {
    let max_c = (1u32 << bits) - 1;
    let loss = 8u32 - u32::from(bits);
    let stepped = (u32::from(v) >> loss).min(max_c);
    (((stepped * 255) + (max_c / 2)) / max_c) as u8
}

fn quantize_channel_f32(v: f32, bits: u8) -> u8 {
    quantize_channel_u8(v.clamp(0.0, 255.0) as u8, bits)
}
