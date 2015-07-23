//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

/// 选图页列表行高（内容像素）。
pub const CHOOSE_MAP_LIST_ROW_H: i32 = 16;

/// 列表框高度可容纳的行数。
pub fn choose_map_visible_rows(list_h: i32) -> usize {
    (list_h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize
}

/// 将滚动偏移钳在 `[0, total.saturating_sub(visible)]`。
pub fn clamp_map_list_scroll(scroll: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    scroll.min(total - visible)
}

/// 调整滚动使 `index` 落在可视窗内（尽量少动）。
pub fn scroll_map_list_to_reveal(scroll: usize, index: usize, total: usize, visible: usize) -> usize {
    if visible == 0 || total == 0 {
        return 0;
    }
    let index = index.min(total - 1);
    let scroll = clamp_map_list_scroll(scroll, total, visible);
    if index < scroll {
        return index;
    }
    if index >= scroll + visible {
        return index + 1 - visible;
    }
    scroll
}

/// 合成选图页：双列表 + 右栏预览 / 使用地图 / 随机 / 取消。
pub fn compose_choose_map_page(
    decoded: &PageDecodeReport,
    viewport_w: u32,
    viewport_h: u32,
    pressed_entry_id: Option<&str>,
    hovered_entry_id: Option<&str>,
    status_text: Option<&str>,
    fnt: Option<&FntFile>,
    csf: Option<&CsfFile>,
    map_preview: Option<&RgbaImage>,
    mode_names: &[&str],
    selected_mode_index: Option<usize>,
    map_names: &[&str],
    selected_map_index: Option<usize>,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let layout = choose_map_layout(viewport_w, viewport_h);
    let mut page = compose_shell_menu_page(
        decoded,
        layout.shell,
        &CHOOSE_MAP_BUTTON_IDS,
        pressed_entry_id,
        hovered_entry_id,
        status_text,
        fnt,
        csf,
        None,
        MenuCaptionKind::ChooseMap,
        wave,
        warn_anim_frame,
    )?;

    blit_skirmish_preview_chrome(&mut page, decoded, layout.shell.panel_top, layout.map_name_plate);
    if let Some(preview) = map_preview {
        blit_map_preview_fit(&mut page, preview, layout.map_preview);
    }

    fill_rect(&mut page, layout.game_type_list, [12, 12, 18, 255]);
    stroke_rect(&mut page, layout.game_type_list, [180, 24, 24, 255]);
    fill_rect(&mut page, layout.map_list, [12, 12, 18, 255]);
    stroke_rect(&mut page, layout.map_list, [180, 24, 24, 255]);

    let visible_modes = (layout.game_type_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (i, name) in mode_names.iter().take(visible_modes).enumerate() {
        let row = RectPx::new(
            layout.game_type_list.x,
            layout.game_type_list.y + (i as i32) * CHOOSE_MAP_LIST_ROW_H,
            layout.game_type_list.w,
            CHOOSE_MAP_LIST_ROW_H,
        );
        if Some(i) == selected_mode_index {
            fill_rect(&mut page, row, [48, 28, 8, 255]);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(&mut page, fnt, name, row.x + 4, row.y, row.w - 8, row.h, MENU_TEXT_ENABLED);
        }
    }

    let visible_rows = (layout.map_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (i, name) in map_names.iter().take(visible_rows).enumerate() {
        let row =
            RectPx::new(layout.map_list.x, layout.map_list.y + (i as i32) * CHOOSE_MAP_LIST_ROW_H, layout.map_list.w, CHOOSE_MAP_LIST_ROW_H);
        if Some(i) == selected_map_index {
            fill_rect(&mut page, row, [48, 28, 8, 255]);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(&mut page, fnt, name, row.x + 4, row.y, row.w - 8, row.h, MENU_TEXT_ENABLED);
        }
    }

    if let Some(fnt) = fnt {
        let title = resolve_caption(csf, "choose_map", Some(choose_map_title_csf_key()));
        blit_shell_static_title(&mut page, fnt, &title, layout.title);
        let engagement = resolve_caption(csf, "select_engagement", choose_map_static_csf_key("select_engagement"));
        blit_text_colored(&mut page, fnt, &engagement, layout.label_engagement.x, layout.label_engagement.y, MENU_TEXT_ENABLED);
        let game_type = resolve_caption(csf, "game_type", choose_map_static_csf_key("game_type"));
        blit_text_colored(&mut page, fnt, &game_type, layout.label_game_type.x, layout.label_game_type.y, MENU_TEXT_ENABLED);
        let game_map = resolve_caption(csf, "game_map", choose_map_static_csf_key("game_map"));
        blit_text_colored(&mut page, fnt, &game_map, layout.label_game_map.x, layout.label_game_map.y, MENU_TEXT_ENABLED);
    }

    Some(page)
}
