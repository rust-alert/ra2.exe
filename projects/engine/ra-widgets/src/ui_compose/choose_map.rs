//! 由一次性拆分自 `ui_compose.rs`。

use super::*;

pub(super) const CHOOSE_MAP_LIST_ROW_H: i32 = 16;

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

    // 游戏类型：Pre-Alpha 仅「作战」一项。
    let type_row = RectPx::new(layout.game_type_list.x, layout.game_type_list.y, layout.game_type_list.w, CHOOSE_MAP_LIST_ROW_H);
    fill_rect(&mut page, type_row, [48, 28, 8, 255]);

    let visible_rows = (layout.map_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (i, name) in map_names.iter().take(visible_rows).enumerate() {
        let row =
            RectPx::new(layout.map_list.x, layout.map_list.y + (i as i32) * CHOOSE_MAP_LIST_ROW_H, layout.map_list.w, CHOOSE_MAP_LIST_ROW_H);
        if Some(i) == selected_map_index {
            fill_rect(&mut page, row, [48, 28, 8, 255]);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(&mut page, fnt, name, row.x + 4, row.y + 1, row.w - 8, row.h - 2, MENU_TEXT_ENABLED);
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
        let battle = resolve_caption(csf, "battle", choose_map_static_csf_key("battle"));
        blit_text_colored(&mut page, fnt, &battle, type_row.x + 4, type_row.y + 1, MENU_TEXT_ENABLED);
    }

    Some(page)
}
