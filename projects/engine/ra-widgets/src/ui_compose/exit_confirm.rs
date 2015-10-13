//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub fn compose_exit_confirm_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    // 主菜单壳与居中 MessageBox 同源一次求解，再压暗并叠对话框。
    let (shell, dlg) = exit_confirm_page_layouts(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        shell,
        &MAIN_MENU_BUTTON_IDS,
        None,
        None,
        None,
        fnt,
        csf,
        movie,
        MenuCaptionKind::Main,
        None,
        warn_anim_frame,
    )?;

    dim_rect(&mut page, shell.canvas, 160);
    if let Some(modal_bg) = find_panel(decoded, "pudlgbgn.shp", 0) {
        blit_rgba(&mut page, &modal_bg.image, dlg.dialog.x, dlg.dialog.y);
    }
    else {
        // 缺底板时不臆造立绘，只留深色框以免完全无反馈。
        fill_rect(&mut page, dlg.dialog, [40, 24, 24, 255]);
    }
    if let Some(fnt) = fnt {
        let prompt = resolve_caption(csf, "exit_confirm", Some(exit_confirm_prompt_csf_key()));
        blit_caption_top_left_clipped(&mut page, fnt, &prompt, dlg.prompt.x, dlg.prompt.y, dlg.prompt.w, dlg.prompt.h, MENU_TEXT_ENABLED);
    }
    let button_ids = &EXIT_CONFIRM_BUTTON_IDS[..];
    let btn_plan = crate::RenderPlan::exit_confirm_placeholders().button_sprite_plan(button_ids);
    btn_plan.paint_sprites_into(&mut page, |slot| {
        resolve_button_sprite(
            decoded,
            slot,
            pressed_entry_id == Some(slot),
            hovered_entry_id == Some(slot),
        )
        .map(|s| &s.image)
    });
    for entry_id in EXIT_CONFIRM_BUTTON_IDS.iter() {
        let Some(cell) = btn_plan.rect_px_of(entry_id) else {
            continue;
        };
        if let Some(fnt) = fnt {
            let key = exit_confirm_csf_label(entry_id);
            let caption = resolve_caption(csf, entry_id, key);
            let pressed = pressed_entry_id == Some(entry_id);
            let (tx, ty, tw, th) = owner_draw_caption_rect(cell, pressed);
            blit_caption_in_cell(&mut page, fnt, &caption, tx, ty, tw, th, MENU_TEXT_ENABLED);
        }
    }
    Some(page)
}
