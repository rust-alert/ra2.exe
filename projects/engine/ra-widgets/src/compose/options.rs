//! 选项页合成。

use super::*;

/// 合成选项页：与主菜单同壳（`mnscrnl` + 右栏波浪）+ 左区控件。
pub fn compose_options_page(
    decoded: &PageDecodeReport,
    state: &crate::options_dialog::OptionsDialogState,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    movie: Option<&RgbaImage>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let _ = (viewport_w, viewport_h);
    let snap = ra_layout::solve_options_page();
    let mut page = compose_shell_menu_page(
        decoded,
        &snap,
        &OPTIONS_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        None,
        fnt,
        csf,
        movie,
        MenuCaptionKind::Options,
        wave,
        warn_anim_frame,
    )?;

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "options", options_dialog_csf_key("title"));
        let title_cell = rect_px_from_snapshot(&snap, "title");
        blit_caption_in_cell(&mut page, fnt, &title, title_cell.x, title_cell.y, title_cell.w, title_cell.h, MENU_TEXT_SECTION);
    }

    paint_options_dialog_controls(&mut page, &snap, state, fnt, csf);
    Some(page)
}
