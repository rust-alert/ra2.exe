//! ???????????? RGBA??? `set_ui_page` ????
//!
//! ?? ? atlas/instance ?????????????????????

use ra_assets::{CsfFile, FntFile};
use ra_renderer::RgbaImage;

use crate::{
    ui_decode::{DecodedUiSprite, PageDecodeReport},
    ui_layout::{
        MAIN_MENU_BUTTON_IDS, MainMenuLayout, RectPx, SINGLE_PLAYER_BUTTON_IDS, main_menu_layout, single_player_layout,
    },
    ui_text::{
        MENU_TEXT_DISABLED, MENU_TEXT_ENABLED, blit_caption_in_cell, main_menu_csf_label, resolve_caption,
        single_player_csf_label,
    },
};

/// Alpha over ? `src` ?? `dst` ? `(x,y)`??????
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

fn blit_stretched(dst: &mut RgbaImage, src: &RgbaImage, rect: RectPx) {
    if rect.w <= 0 || rect.h <= 0 || src.width() == 0 || src.height() == 0 {
        return;
    }
    // ???????/??????????????????
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
            if src.as_raw()[si + 3] == 0 {
                continue;
            }
            let di = ((dy as u32 * dst.width() + dx as u32) * 4) as usize;
            dst.as_mut()[di..di + 4].copy_from_slice(&src.as_raw()[si..si + 4]);
        }
    }
}

fn find_panel<'a>(decoded: &'a PageDecodeReport, needle: &str) -> Option<&'a DecodedUiSprite> {
    decoded.panels.iter().find(|p| p.label.to_ascii_lowercase().starts_with(&needle.to_ascii_lowercase()))
}

fn find_button_normal<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_normals.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

fn find_button_pressed<'a>(decoded: &'a PageDecodeReport, entry_id: &str) -> Option<&'a DecodedUiSprite> {
    decoded.button_presseds.iter().find(|(id, _)| *id == entry_id).map(|(_, sprite)| sprite)
}

fn compose_shell_menu_page(
    decoded: &PageDecodeReport,
    layout: MainMenuLayout,
    button_ids: &[&str],
    pressed_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    single_player: bool,
) -> Option<RgbaImage> {
    let bg = decoded.background.as_ref()?;
    let mut page = RgbaImage::from_raw(
        layout.canvas.w as u32,
        layout.canvas.h as u32,
        vec![0u8; (layout.canvas.w as usize) * (layout.canvas.h as usize) * 4],
    )?;

    blit_rgba(&mut page, &bg.image, layout.background.x, layout.background.y);
    if let Some(frame) = movie {
        blit_stretched(&mut page, frame, layout.movie);
    }

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

    for (i, entry_id) in button_ids.iter().enumerate() {
        let normal = find_button_normal(decoded, entry_id)?;
        let sprite =
            if pressed_entry_id == Some(*entry_id) { find_button_pressed(decoded, entry_id).unwrap_or(normal) } else { normal };
        let cell = layout.buttons[i];
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        if let Some(fnt) = fnt {
            let key = if single_player { single_player_csf_label(entry_id) } else { main_menu_csf_label(entry_id) };
            let caption = resolve_caption(csf, entry_id, key);
            let disabled = matches!(*entry_id, "network" | "campaign" | "training");
            let color = if disabled { MENU_TEXT_DISABLED } else { MENU_TEXT_ENABLED };
            blit_caption_in_cell(&mut page, fnt, &caption, cell.x, cell.y, cell.w, cell.h, color);
        }
    }

    Some(page)
}

/// ????? chrome?
pub fn compose_main_menu_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
) -> Option<RgbaImage> {
    compose_shell_menu_page(
        decoded,
        main_menu_layout(viewport_w, viewport_h),
        &MAIN_MENU_BUTTON_IDS,
        pressed_entry_id,
        fnt,
        csf,
        movie,
        false,
    )
}

/// ??????? chrome?
pub fn compose_single_player_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
) -> Option<RgbaImage> {
    compose_shell_menu_page(
        decoded,
        single_player_layout(viewport_w, viewport_h),
        &SINGLE_PLAYER_BUTTON_IDS,
        pressed_entry_id,
        fnt,
        csf,
        movie,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blit_writes_opaque_pixel() {
        let mut dst = RgbaImage::from_raw(2, 2, vec![0u8; 16]).unwrap();
        let src = RgbaImage::from_raw(1, 1, vec![10, 20, 30, 255]).unwrap();
        blit_rgba(&mut dst, &src, 1, 1);
        assert_eq!(&dst.as_mut()[12..16], &[10, 20, 30, 255]);
    }

    fn solid_sprite(label: &str, rgba: [u8; 4]) -> DecodedUiSprite {
        DecodedUiSprite {
            label: label.into(),
            image: RgbaImage::from_raw(1, 1, rgba.to_vec()).unwrap(),
            origin: "test".into(),
            frame: 0,
            canvas: (1, 1),
            frame_rect: (0, 0, 1, 1),
        }
    }

    #[test]
    fn compose_uses_pressed_sprite_when_entry_matches() {
        let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
        let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
        let pressed = solid_sprite("sdbtnanm.shp#4", [200, 0, 0, 255]);
        let decoded = PageDecodeReport {
            background: Some(bg),
            panels: Vec::new(),
            button_normals: MAIN_MENU_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
            button_presseds: vec![("single_player", pressed)],
            errors: Vec::new(),
        };
        let page = compose_main_menu_page(&decoded, 800, 600, Some("single_player"), None, None, None).unwrap();
        let layout = main_menu_layout(800, 600);
        let cell = layout.buttons[0];
        let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
        assert_eq!(&page.as_raw()[di..di + 4], &[200, 0, 0, 255]);
    }

    #[test]
    fn compose_single_player_uses_skirmish_id() {
        let bg = solid_sprite("mnscrnl.shp#0", [1, 2, 3, 255]);
        let normal = solid_sprite("sdbtnanm.shp#2", [10, 10, 10, 255]);
        let pressed = solid_sprite("sdbtnanm.shp#4", [0, 200, 0, 255]);
        let decoded = PageDecodeReport {
            background: Some(bg),
            panels: Vec::new(),
            button_normals: SINGLE_PLAYER_BUTTON_IDS.iter().map(|id| (*id, normal.clone())).collect(),
            button_presseds: vec![("skirmish", pressed)],
            errors: Vec::new(),
        };
        let page = compose_single_player_page(&decoded, 800, 600, Some("skirmish"), None, None, None).unwrap();
        let layout = single_player_layout(800, 600);
        let cell = layout.buttons[1];
        let di = ((cell.y as u32 * page.width() + cell.x as u32) * 4) as usize;
        assert_eq!(&page.as_raw()[di..di + 4], &[0, 200, 0, 255]);
    }
}
