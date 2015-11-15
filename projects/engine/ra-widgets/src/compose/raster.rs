//! RGBA blit 与基础栅格辅助。

use super::*;

pub fn blit_rgba(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    if src.width() == 0 || src.height() == 0 || dst.width() == 0 || dst.height() == 0 {
        return;
    }
    for row in 0..src.height() {
        let dy = y + row as i32;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..src.width() {
            let dx = x + col as i32;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let si = ((row * src.width() + col) * 4) as usize;
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            let sa = src.as_raw()[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
                continue;
            }
            let inv = 255 - sa;
            for c in 0..3 {
                let s = src.as_raw()[si + c] as u32;
                let d = dst.as_mut()[di + c] as u32;
                dst.as_mut()[di + c] = ((s * sa + d * inv) / 255) as u8;
            }
            dst.as_mut()[di + 3] = 255;
        }
    }
}

pub(super) fn blit_stretched(dst: &mut RgbaImage, src: &RgbaImage, rect: RectPx) {
    if rect.w <= 0 || rect.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    // 面板条允许纵向/横向铺满目标格；用最近邻，避免模糊。
    for row in 0..rect.h as u32 {
        let sy = row * src.height() / rect.h as u32;
        for col in 0..rect.w as u32 {
            let sx = col * src.width() / rect.w as u32;
            let si = ((sy * src.width() + sx) * 4) as usize;
            let dx = rect.x + col as i32;
            let dy = rect.y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width() || dy as u32 >= dst.height() {
                continue;
            }
            let raw = src.as_raw();
            if raw[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 地图预览：在 `0x468` 槽内等比适配（整数 `*1000` 缩放），不拉满、不描边。
pub(super) fn blit_map_preview_fit(dst: &mut RgbaImage, src: &RgbaImage, slot: RectPx) {
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    if sw <= 0 || sh <= 0 || slot.w <= 0 || slot.h <= 0 {
        return;
    }
    let scale_w = (slot.w * 1000) / sw;
    let scale_h = (slot.h * 1000) / sh;
    let scale = scale_w.min(scale_h).max(1);
    let fit_w = (sw * scale) / 1000;
    let fit_h = (sh * scale) / 1000;
    let fit_x = slot.x + slot.w / 2 - (sw * scale) / 2000;
    let fit_y = slot.y + slot.h / 2 - (sh * scale) / 2000;
    blit_stretched(dst, src, RectPx::new(fit_x, fit_y, fit_w.max(1), fit_h.max(1)));
}

/// 1:1 贴图，跳过透明与近黑（`fsscrn` 空区约 (8,8,8)，非索引 0）。
pub(super) fn blit_rgba_skip_near_black(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, max_rgb_sum: u16) {
    let raw = src.as_raw();
    for row in 0..src.height() {
        for col in 0..src.width() {
            let si = ((row * src.width() + col) * 4) as usize;
            if raw[si + 3] == 0 {
                continue;
            }
            let sum = u16::from(raw[si]) + u16::from(raw[si + 1]) + u16::from(raw[si + 2]);
            if sum <= max_rgb_sum {
                continue;
            }
            let dx = x + col as i32;
            let dy = y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width() || dy as u32 >= dst.height() {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

/// 相对静止帧差分贴图：只画动画帧相对 `base` 变化的像素（战役悬停箭头）。
pub(super) fn blit_rgba_diff_from_base(dst: &mut RgbaImage, src: &RgbaImage, base: Option<&RgbaImage>, x: i32, y: i32, max_rgb_sum: u16) {
    let Some(base) = base
    else {
        blit_rgba_skip_near_black(dst, src, x, y, max_rgb_sum);
        return;
    };
    if src.width() != base.width() || src.height() != base.height() {
        blit_rgba_skip_near_black(dst, src, x, y, max_rgb_sum);
        return;
    }
    let raw = src.as_raw();
    let base_raw = base.as_raw();
    for row in 0..src.height() {
        for col in 0..src.width() {
            let si = ((row * src.width() + col) * 4) as usize;
            if raw[si + 3] == 0 {
                continue;
            }
            let sum = u16::from(raw[si]) + u16::from(raw[si + 1]) + u16::from(raw[si + 2]);
            if sum <= max_rgb_sum {
                continue;
            }
            let dr = i16::from(raw[si]).abs_diff(i16::from(base_raw[si]));
            let dg = i16::from(raw[si + 1]).abs_diff(i16::from(base_raw[si + 1]));
            let db = i16::from(raw[si + 2]).abs_diff(i16::from(base_raw[si + 2]));
            if u16::from(dr) + u16::from(dg) + u16::from(db) < 24 {
                continue;
            }
            let dx = x + col as i32;
            let dy = y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width() || dy as u32 >= dst.height() {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&raw[si..si + 4]);
        }
    }
}

pub(super) fn fill_rect(dst: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    for row in 0..rect.h {
        let dy = rect.y + row;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..rect.w {
            let dx = rect.x + col;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&rgba);
        }
    }
}

/// 将矩形区域整体压暗（`amount` 越大越暗，0..255）。
pub(super) fn dim_rect(dst: &mut RgbaImage, rect: RectPx, amount: u8) {
    if rect.w <= 0 || rect.h <= 0 || amount == 0 {
        return;
    }
    let keep = 255u32.saturating_sub(u32::from(amount));
    for row in 0..rect.h {
        let dy = rect.y + row;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..rect.w {
            let dx = rect.x + col;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            for c in 0..3 {
                let v = u32::from(dst.as_raw()[di + c]);
                dst.as_mut()[di + c] = ((v * keep) / 255) as u8;
            }
        }
    }
}


pub(super) fn stroke_rect(dst: &mut RgbaImage, rect: RectPx, rgba: [u8; 4]) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    fill_rect(dst, RectPx::new(rect.x, rect.y, rect.w, 1), rgba);
    fill_rect(dst, RectPx::new(rect.x, rect.y + rect.h - 1, rect.w, 1), rgba);
    fill_rect(dst, RectPx::new(rect.x, rect.y, 1, rect.h), rgba);
    fill_rect(dst, RectPx::new(rect.x + rect.w - 1, rect.y, 1, rect.h), rgba);
}

/// 按源图宽度裁剪后 1:1 贴图（装载进度条横向揭示）。
pub(super) fn blit_rgba_clipped_width(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, clip_w: u32) {
    let w = clip_w.min(src.width());
    if w == 0 || src.height() == 0 {
        return;
    }
    for row in 0..src.height() {
        let dy = y + row as i32;
        if dy < 0 || dy as u32 >= dst.height() {
            continue;
        }
        for col in 0..w {
            let dx = x + col as i32;
            if dx < 0 || dx as u32 >= dst.width() {
                continue;
            }
            let si = ((row * src.width() + col) * 4) as usize;
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            let sa = src.as_raw()[si + 3];
            if sa == 0 {
                continue;
            }
            dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
        }
    }
}
