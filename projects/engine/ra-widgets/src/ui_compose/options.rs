//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub fn compose_options_page(
    decoded: &PageDecodeReport,
    state: &crate::options_dialog::OptionsDialogState,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    _movie: Option<&RgbaImage>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let _ = (viewport_w, viewport_h);
    // 整页几何只求一次：chrome、右栏三钮与内容板同源。
    let dlg = crate::options_dialog::OptionsDialogLayout::new();
    let mut page = RgbaImage::from_raw(
        dlg.canvas.w as u32,
        dlg.canvas.h as u32,
        vec![0u8; (dlg.canvas.w as usize) * (dlg.canvas.h as usize) * 4],
    )?;
    // 整页黑底，避免残留主菜单影片/大背景。
    fill_rect(&mut page, dlg.canvas, [0, 0, 0, 255]);

    blit_right_panel_top(&mut page, decoded, dlg.panel_top, warn_anim_frame);
    if let Some(tile) = find_panel(decoded, "sdbtnbkgd.shp", 0) {
        for i in 0..dlg.panel_tile_count {
            let r = RectPx::new(
                dlg.panel_tile.x,
                dlg.panel_tile.y + i * dlg.panel_tile.h,
                dlg.panel_tile.w,
                dlg.panel_tile.h,
            );
            blit_stretched(&mut page, &tile.image, r);
        }
    }
    if let Some(bottom) = find_panel(decoded, "sdbtm.shp", 0) {
        blit_stretched(&mut page, &bottom.image, dlg.panel_bottom);
    }
    if let Some(lower) = find_panel(decoded, "lwscrnl.shp", 0) {
        blit_stretched(&mut page, &lower.image, dlg.lower_strip);
    }

    for (i, entry_id) in OPTIONS_BUTTON_IDS.iter().enumerate() {
        let Some(normal) = find_button_normal(decoded, entry_id) else {
            continue;
        };
        let sprite = if pressed_entry_id == Some(entry_id) {
            find_button_pressed(decoded, entry_id).unwrap_or(normal)
        } else if hovered_entry_id == Some(entry_id) {
            find_button_hover(decoded, entry_id).unwrap_or(normal)
        } else {
            normal
        };
        let cell = dlg.rail[i];
        blit_rgba(&mut page, &sprite.image, cell.x, cell.y);
        if let Some(fnt) = fnt {
            let key = options_csf_label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let pressed = pressed_entry_id == Some(entry_id);
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "options", options_dialog_csf_key("title"));
        blit_caption_in_cell(
            &mut page,
            fnt,
            &title,
            dlg.title.x,
            dlg.title.y,
            dlg.title.w,
            dlg.title.h,
            MENU_TEXT_SECTION,
        );
    }

    paint_options_dialog_controls(&mut page, &dlg, state, fnt, csf);
    Some(page)
}
