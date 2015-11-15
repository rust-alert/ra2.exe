//! 选项页合成。

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
    let snap = ra_layout::solve_options_page();
    let canvas = RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H);
    let mut page = RgbaImage::from_raw(
        canvas.w as u32,
        canvas.h as u32,
        vec![0u8; (canvas.w as usize) * (canvas.h as usize) * 4],
    )?;
    // 整页黑底，避免残留主菜单影片/大背景。
    fill_rect(&mut page, canvas, [0, 0, 0, 255]);

    paint_right_panel_chrome(
        &mut page,
        decoded,
        rect_px_from_snapshot(&snap, "panel_top"),
        rect_px_from_snapshot(&snap, "panel_tile"),
        panel_tile_count_from_snap(&snap),
        rect_px_from_snapshot(&snap, "panel_bottom"),
        rect_px_from_snapshot(&snap, "lower_strip"),
        warn_anim_frame,
    );

    let button_ids = &OPTIONS_BUTTON_IDS[..];
    let btn_plan = crate::RenderPlan::options_page_placeholders().button_sprite_plan(button_ids);
    btn_plan.paint_sprites_into(&mut page, |slot| {
        resolve_button_sprite(
            decoded,
            slot,
            pressed_entry_id == Some(slot),
            hovered_entry_id == Some(slot),
        )
        .map(|s| &s.image)
    });
    for entry_id in OPTIONS_BUTTON_IDS.iter() {
        let Some(cell) = btn_plan.rect_px_of(entry_id) else {
            continue;
        };
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
        let title_cell = rect_px_from_snapshot(&snap, "title");
        blit_caption_in_cell(
            &mut page,
            fnt,
            &title,
            title_cell.x,
            title_cell.y,
            title_cell.w,
            title_cell.h,
            MENU_TEXT_SECTION,
        );
    }

    paint_options_dialog_controls(&mut page, &snap, state, fnt, csf);
    Some(page)
}
