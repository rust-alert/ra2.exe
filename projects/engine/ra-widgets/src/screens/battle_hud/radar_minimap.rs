//! 对局雷达槽：俯视格网小地图合成与适配。

use ra_layout::RectPx;
use ra_renderer::RgbaImage;
use ra_types::LandType;

/// 雷达内容相对槽的内缩（避开金属边框）。
pub const RADAR_CONTENT_INSET: i32 = 6;

/// 陆地类型 → 雷达底色（俯视示意，非原版调色板逐像素复刻）。
pub fn land_type_radar_rgba(land: LandType) -> [u8; 4] {
    match land {
        LandType::Clear => [34, 72, 28, 255],
        LandType::Road => [96, 88, 64, 255],
        LandType::Water => [28, 56, 140, 255],
        LandType::Rock => [72, 72, 72, 255],
        LandType::Wall => [24, 24, 24, 255],
        LandType::Tiberium => [180, 200, 40, 255],
        LandType::Beach => [168, 148, 88, 255],
        LandType::Rough => [56, 80, 36, 255],
        LandType::Ice => [180, 200, 220, 255],
        LandType::Railroad => [64, 64, 72, 255],
        LandType::Tunnel => [40, 40, 48, 255],
        LandType::Weeds => [96, 48, 120, 255],
    }
}

/// 单位 / 建筑色点。
#[derive(Debug, Clone, Copy)]
pub struct RadarMinimapBlip {
    pub x: u16,
    pub y: u16,
    pub rgba: [u8; 4],
    /// 建筑画 2×2，机动单位 1×1。
    pub structure: bool,
}

/// 由陆地序号行主序表与色点合成俯视小地图（1 格 = 1 像素）。
///
/// `land_types` 长度须为 `width * height`，元素为 [`LandType`] 序号。
/// `view` 为可选镜头可视格 AABB（含端点），描亮黄框。
pub fn compose_radar_minimap(
    width: u32,
    height: u32,
    land_types: &[u8],
    blips: &[RadarMinimapBlip],
    view: Option<(u16, u16, u16, u16)>,
) -> Option<RgbaImage> {
    let w = width.max(1);
    let h = height.max(1);
    let need = (w as usize).saturating_mul(h as usize);
    if land_types.len() < need {
        return None;
    }
    let mut pixels = vec![0u8; need * 4];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let land = LandType::from_u8(land_types[i]).unwrap_or(LandType::Clear);
            let rgba = land_type_radar_rgba(land);
            let di = i * 4;
            pixels[di..di + 4].copy_from_slice(&rgba);
        }
    }
    let put = |pixels: &mut [u8], x: i32, y: i32, rgba: [u8; 4]| {
        if x < 0 || y < 0 || x as u32 >= w || y as u32 >= h {
            return;
        }
        let di = ((y as u32 * w + x as u32) * 4) as usize;
        pixels[di..di + 4].copy_from_slice(&rgba);
    };
    for blip in blips {
        let x = i32::from(blip.x);
        let y = i32::from(blip.y);
        put(&mut pixels, x, y, blip.rgba);
        if blip.structure {
            put(&mut pixels, x + 1, y, blip.rgba);
            put(&mut pixels, x, y + 1, blip.rgba);
            put(&mut pixels, x + 1, y + 1, blip.rgba);
        }
    }
    if let Some((x0, y0, x1, y1)) = view {
        let left = x0.min(x1);
        let right = x0.max(x1);
        let top = y0.min(y1);
        let bottom = y0.max(y1);
        let edge = [255, 220, 64, 255];
        for x in left..=right {
            put(&mut pixels, i32::from(x), i32::from(top), edge);
            put(&mut pixels, i32::from(x), i32::from(bottom), edge);
        }
        for y in top..=bottom {
            put(&mut pixels, i32::from(left), i32::from(y), edge);
            put(&mut pixels, i32::from(right), i32::from(y), edge);
        }
    }
    RgbaImage::from_raw(w, h, pixels)
}

/// 雷达槽内内容区（内缩后）。
pub fn radar_content_rect(slot: RectPx) -> RectPx {
    let inset = RADAR_CONTENT_INSET;
    let w = (slot.w - inset * 2).max(1);
    let h = (slot.h - inset * 2).max(1);
    RectPx::new(slot.x + inset, slot.y + inset, w, h)
}

/// 小地图在内容区内的等比适配矩形（与选图预览同一整数缩放口径）。
pub fn radar_minimap_fit_rect(src_w: u32, src_h: u32, content: RectPx) -> RectPx {
    let sw = src_w as i32;
    let sh = src_h as i32;
    if sw <= 0 || sh <= 0 || content.w <= 0 || content.h <= 0 {
        return RectPx::new(content.x, content.y, 1, 1);
    }
    let scale_w = (content.w * 1000) / sw;
    let scale_h = (content.h * 1000) / sh;
    let scale = scale_w.min(scale_h).max(1);
    let fit_w = ((sw * scale) / 1000).max(1);
    let fit_h = ((sh * scale) / 1000).max(1);
    let fit_x = content.x + content.w / 2 - (sw * scale) / 2000;
    let fit_y = content.y + content.h / 2 - (sh * scale) / 2000;
    RectPx::new(fit_x, fit_y, fit_w, fit_h)
}

/// 屏幕点映射到地图格（适配矩形内）；点在框外返回 `None`。
pub fn radar_fit_xy_to_cell(fit: RectPx, map_w: u32, map_h: u32, px: i32, py: i32) -> Option<(u16, u16)> {
    if fit.w <= 0 || fit.h <= 0 || map_w == 0 || map_h == 0 {
        return None;
    }
    if px < fit.x || py < fit.y || px >= fit.x + fit.w || py >= fit.y + fit.h {
        return None;
    }
    let lx = px - fit.x;
    let ly = py - fit.y;
    let cx = ((lx as i64) * map_w as i64 / fit.w as i64) as u32;
    let cy = ((ly as i64) * map_h as i64 / fit.h as i64) as u32;
    Some((cx.min(map_w - 1) as u16, cy.min(map_h - 1) as u16))
}

/// 把小地图等比贴进雷达内容区（先铺深底）。
pub fn blit_radar_minimap_into_slot(dst: &mut RgbaImage, src: &RgbaImage, slot: RectPx) {
    let content = radar_content_rect(slot);
    // 开图后雷达窗底色。
    for y in content.y..content.y + content.h {
        if y < 0 {
            continue;
        }
        let yu = y as u32;
        if yu >= dst.height() {
            break;
        }
        for x in content.x..content.x + content.w {
            if x < 0 {
                continue;
            }
            let xu = x as u32;
            if xu >= dst.width() {
                break;
            }
            let di = ((yu * dst.width() + xu) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&[8, 12, 20, 255]);
        }
    }
    let fit = radar_minimap_fit_rect(src.width(), src.height(), content);
    if fit.w <= 0 || fit.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    for row in 0..fit.h as u32 {
        let sy = row * src.height() / fit.h as u32;
        for col in 0..fit.w as u32 {
            let sx = col * src.width() / fit.w as u32;
            let si = ((sy * src.width() + sx) * 4) as usize;
            let dx = fit.x + col as i32;
            let dy = fit.y + row as i32;
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
