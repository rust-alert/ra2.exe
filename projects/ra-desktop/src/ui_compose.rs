//! 将已解码壳层精灵合成整页 RGBA（上传 `set_ui_page` 之前）。
//!
//! 合成 ≠ atlas/instance 终态；当前只为验证颜色、原尺寸与粗略位置。

use ra_renderer::RgbaImage;

use crate::{
    ui_decode::{DecodedUiSprite, PageDecodeReport},
    ui_layout::{RectPx, main_menu_layout},
};

/// Alpha over 将 `src` 画到 `dst` 的 `(x,y)`（可裁剪）。
pub fn blit_rgba(dst: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32) {
    if src.width == 0 || src.height == 0 || dst.width == 0 || dst.height == 0 {
        return;
    }
    for row in 0..src.height {
        let dy = y + row as i32;
        if dy < 0 || dy as u32 >= dst.height {
            continue;
        }
        for col in 0..src.width {
            let dx = x + col as i32;
            if dx < 0 || dx as u32 >= dst.width {
                continue;
            }
            let si = ((row * src.width + col) * 4) as usize;
            let di = ((dy as u32 * dst.width + dx as u32) * 4) as usize;
            let sa = src.pixels[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst.pixels[di..di + 4].copy_from_slice(&src.pixels[si..si + 4]);
                continue;
            }
            let inv = 255 - sa;
            for c in 0..3 {
                let s = src.pixels[si + c] as u32;
                let d = dst.pixels[di + c] as u32;
                dst.pixels[di + c] = ((s * sa + d * inv) / 255) as u8;
            }
            dst.pixels[di + 3] = 255;
        }
    }
}

fn blit_stretched(dst: &mut RgbaImage, src: &RgbaImage, rect: RectPx) {
    if rect.w <= 0 || rect.h <= 0 || src.width == 0 || src.height == 0 {
        return;
    }
    // 面板条允许纵向/横向铺满目标格；用最近邻，避免模糊。
    for row in 0..rect.h as u32 {
        let sy = row * src.height / rect.h as u32;
        for col in 0..rect.w as u32 {
            let sx = col * src.width / rect.w as u32;
            let si = ((sy * src.width + sx) * 4) as usize;
            let dx = rect.x + col as i32;
            let dy = rect.y + row as i32;
            if dx < 0 || dy < 0 || dx as u32 >= dst.width || dy as u32 >= dst.height {
                continue;
            }
            if src.pixels[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width + dx as u32) * 4) as usize;
            dst.pixels[di..di + 4].copy_from_slice(&src.pixels[si..si + 4]);
        }
    }
}

fn find_panel<'a>(decoded: &'a PageDecodeReport, needle: &str) -> Option<&'a DecodedUiSprite> {
    decoded
        .panels
        .iter()
        .find(|p| p.label.to_ascii_lowercase().starts_with(&needle.to_ascii_lowercase()))
}

/// 合成主菜单静态 chrome：背景 + 右侧板 + 首个可点按钮常态。
///
/// 缺背景或首钮时返回 `None`（该页视觉验收不得通过）。
pub fn compose_main_menu_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
) -> Option<RgbaImage> {
    let bg = decoded.background.as_ref()?;
    let layout = main_menu_layout(viewport_w, viewport_h);
    let mut page = RgbaImage::new(
        layout.canvas.w as u32,
        layout.canvas.h as u32,
        vec![0u8; (layout.canvas.w as usize) * (layout.canvas.h as usize) * 4],
    )?;

    // 背景：原尺寸左上对齐；若大于画布则左上裁剪，若小于则居中。
    let bg_x = if bg.image.width as i32 >= layout.canvas.w {
        0
    } else {
        (layout.canvas.w - bg.image.width as i32) / 2
    };
    let bg_y = if bg.image.height as i32 >= layout.canvas.h {
        0
    } else {
        (layout.canvas.h - bg.image.height as i32) / 2
    };
    blit_rgba(&mut page, &bg.image, bg_x, bg_y);

    if let Some(top) = find_panel(decoded, "sdtp.shp") {
        blit_stretched(&mut page, &top.image, layout.panel_top);
    }
    if let Some(tile) = find_panel(decoded, "sdbtnbkgd.shp") {
        for i in 0..layout.panel_tile_count {
            let r = RectPx::new(
                layout.panel_tile.x,
                layout.panel_tile.y + i * layout.panel_tile.h,
                layout.panel_tile.w,
                layout.panel_tile.h,
            );
            blit_stretched(&mut page, &tile.image, r);
        }
    }
    if let Some(bottom) = find_panel(decoded, "sdbtm.shp") {
        blit_stretched(&mut page, &bottom.image, layout.panel_bottom);
    }
    if let Some(lower) = find_panel(decoded, "lwscrnl.shp") {
        blit_stretched(&mut page, &lower.image, layout.lower_strip);
    }

    // 只画第一个可点按钮（单人），验证颜色与尺寸。
    let btn = decoded
        .button_normals
        .iter()
        .find(|(id, _)| *id == "single_player")
        .map(|(_, s)| s)?;
    // 按钮保持原尺寸，锚定在格左上（不拉伸）。
    blit_rgba(&mut page, &btn.image, layout.first_button.x, layout.first_button.y);

    let _ = layout;
    Some(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blit_writes_opaque_pixel() {
        let mut dst = RgbaImage::new(2, 2, vec![0u8; 16]).unwrap();
        let src = RgbaImage::new(1, 1, vec![10, 20, 30, 255]).unwrap();
        blit_rgba(&mut dst, &src, 1, 1);
        assert_eq!(&dst.pixels[12..16], &[10, 20, 30, 255]);
    }
}
