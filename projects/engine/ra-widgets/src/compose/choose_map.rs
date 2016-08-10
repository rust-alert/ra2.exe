//! 选图页合成。

use super::*;

/// 选图列表选中行底色（`DAT_00AC4604 = 0xFF` → 源 RGB 纯红）。
pub const CHOOSE_MAP_LIST_SELECTED: [u8; 4] = [255, 0, 0, 255];

/// 选图列表框描边。
pub const CHOOSE_MAP_LIST_BORDER: [u8; 4] = [255, 0, 0, 255];

/// 选图列表底。
pub const CHOOSE_MAP_LIST_BG: [u8; 4] = [0, 0, 0, 255];

/// 选图列表行文字色（壳层黄 `DAT_00AC18A4 = 0x0000FFFF`）。
pub const CHOOSE_MAP_LIST_TEXT: [u8; 4] = [255, 255, 0, 255];

/// 选图页滚动条槽。
pub const CHOOSE_MAP_SCROLL_TRACK: [u8; 4] = [48, 8, 8, 255];
/// 选图页滚动条拇指 / 箭头。
pub const CHOOSE_MAP_SCROLL_THUMB: [u8; 4] = [255, 0, 0, 255];

/// 溢出时列表内容区右侧留给滚动条的宽度（`1*2+0x12`）。
pub const CHOOSE_MAP_SCROLL_W: i32 = 20;

/// 合成选图页：双列表 + 右栏预览 / 模式·地图名 / 使用地图 / 随机 / 取消。
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
    selected_mode_caption: Option<&str>,
    selected_map_caption: Option<&str>,
    mode_names: &[&str],
    selected_mode_index: Option<usize>,
    map_names: &[&str],
    selected_map_index: Option<usize>,
    map_list_scroll: usize,
    wave: Option<ShellWaveFrames<'_>>,
    warn_anim_frame: usize,
) -> Option<RgbaImage> {
    let _ = (viewport_w, viewport_h);
    let snap = ra_layout::solve_choose_map();
    let panel_top = rect_px_from_snapshot(&snap, "panel_top");
    let title = rect_px_from_snapshot(&snap, "title");
    let map_preview_rect = rect_px_from_snapshot(&snap, "map_preview");
    let map_name_plate = rect_px_from_snapshot(&snap, "map_name_plate");
    let game_type = rect_px_from_snapshot(&snap, "game_type");
    let map_label = rect_px_from_snapshot(&snap, "map_label");
    let label_engagement = rect_px_from_snapshot(&snap, "label_engagement");
    let label_game_type = rect_px_from_snapshot(&snap, "label_game_type");
    let label_game_map = rect_px_from_snapshot(&snap, "label_game_map");
    let game_type_list = rect_px_from_snapshot(&snap, "game_type_list");
    let map_list = rect_px_from_snapshot(&snap, "map_list");

    let mut page = compose_shell_menu_page(
        decoded,
        &snap,
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

    blit_skirmish_preview_chrome(&mut page, decoded, panel_top, map_name_plate);
    // 钮面画在底板之后，保证与 `map_name_plate` 相接时不被挡板盖住。
    let btn_plan = shell_button_sprite_plan(MenuCaptionKind::ChooseMap, &CHOOSE_MAP_BUTTON_IDS);
    paint_shell_rail_buttons(
        &mut page,
        decoded,
        &CHOOSE_MAP_BUTTON_IDS,
        &btn_plan,
        pressed_entry_id,
        hovered_entry_id,
        fnt,
        csf,
        MenuCaptionKind::ChooseMap,
        wave,
    )?;
    if let Some(preview) = map_preview {
        blit_map_preview_fit(&mut page, preview, map_preview_rect);
    }
    // 右栏信息板：与遭遇战同槽，模式名 / 地图名格内居中（板面只留 chrome）。
    if let Some(fnt) = fnt {
        if let Some(caption) = selected_mode_caption.filter(|s| !s.is_empty()) {
            blit_caption_in_cell(&mut page, fnt, caption, game_type.x, game_type.y, game_type.w, game_type.h, MENU_TEXT_ENABLED);
        }
        if let Some(caption) = selected_map_caption.filter(|s| !s.is_empty()) {
            blit_caption_in_cell(&mut page, fnt, caption, map_label.x, map_label.y, map_label.w, map_label.h, MENU_TEXT_ENABLED);
        }
    }

    fill_rect(&mut page, game_type_list, CHOOSE_MAP_LIST_BG);
    stroke_rect(&mut page, game_type_list, CHOOSE_MAP_LIST_BORDER);
    fill_rect(&mut page, map_list, CHOOSE_MAP_LIST_BG);
    stroke_rect(&mut page, map_list, CHOOSE_MAP_LIST_BORDER);

    let visible_modes = (game_type_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (i, name) in mode_names.iter().take(visible_modes).enumerate() {
        let row = choose_map_list_row_rect(game_type_list, i, game_type_list.w);
        if Some(i) == selected_mode_index {
            fill_rect(&mut page, row, CHOOSE_MAP_LIST_SELECTED);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(&mut page, fnt, name, row.x + 2, row.y, row.w - 4, row.h, CHOOSE_MAP_LIST_TEXT);
        }
    }

    let visible_rows = choose_map_visible_rows(map_list.h);
    let scroll = clamp_map_list_scroll(map_list_scroll, map_names.len(), visible_rows);
    let map_overflow = map_names.len() > visible_rows && visible_rows > 0;
    let map_content_w = if map_overflow { (map_list.w - CHOOSE_MAP_SCROLL_W).max(8) } else { map_list.w };
    for (row_i, name) in map_names.iter().skip(scroll).take(visible_rows).enumerate() {
        let abs_i = scroll + row_i;
        let row = choose_map_list_row_rect(map_list, row_i, map_content_w);
        if Some(abs_i) == selected_map_index {
            fill_rect(&mut page, row, CHOOSE_MAP_LIST_SELECTED);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(&mut page, fnt, name, row.x + 2, row.y, (row.w - 4).max(8), row.h, CHOOSE_MAP_LIST_TEXT);
        }
    }

    paint_choose_map_scrollbar(&mut page, map_list, map_names.len(), visible_rows, scroll);

    let status_help = rect_px_from_snapshot(&snap, "status_help");
    if let Some(fnt) = fnt {
        let title_text = resolve_caption(csf, "choose_map", Some(choose_map_title_csf_key()));
        blit_shell_static_title(&mut page, fnt, &title_text, title);
        let engagement = resolve_caption(csf, "select_engagement", choose_map_static_csf_key("select_engagement"));
        blit_caption_in_cell(
            &mut page,
            fnt,
            &engagement,
            label_engagement.x,
            label_engagement.y,
            label_engagement.w,
            label_engagement.h,
            MENU_TEXT_ENABLED,
        );
        let game_type_hdr = resolve_caption(csf, "game_type", choose_map_static_csf_key("game_type"));
        blit_text_colored(&mut page, fnt, &game_type_hdr, label_game_type.x, label_game_type.y, MENU_TEXT_ENABLED);
        let game_map = resolve_caption(csf, "game_map", choose_map_static_csf_key("game_map"));
        blit_text_colored(&mut page, fnt, &game_map, label_game_map.x, label_game_map.y, MENU_TEXT_ENABLED);
        if let Some(text) = status_text.filter(|s| !s.is_empty()) {
            blit_text_colored(&mut page, fnt, text, status_help.x, status_help.y, MENU_TEXT_ENABLED);
        }
    }

    Some(page)
}

fn paint_choose_map_scrollbar(page: &mut RgbaImage, list: RectPx, total: usize, visible: usize, scroll: usize) {
    if total <= visible || visible == 0 || list.h <= 8 {
        return;
    }
    let track = RectPx::new(list.x + list.w - CHOOSE_MAP_SCROLL_W, list.y, CHOOSE_MAP_SCROLL_W, list.h);
    fill_rect(page, track, CHOOSE_MAP_SCROLL_TRACK);
    stroke_rect(page, track, CHOOSE_MAP_LIST_BORDER);
    let max_scroll = total - visible;
    let arrow = 8i32;
    let travel_h = (track.h - arrow * 2).max(1);
    let thumb_h = ((travel_h as usize * visible) / total).max(12).min(travel_h as usize) as i32;
    let travel = (travel_h - thumb_h).max(0);
    let thumb_y = track.y + arrow + ((travel as usize * scroll) / max_scroll.max(1)) as i32;
    fill_rect(page, RectPx::new(track.x + 2, thumb_y, track.w - 4, thumb_h), CHOOSE_MAP_SCROLL_THUMB);
    let mid_x = track.x + track.w / 2;
    for (i, dy) in [0i32, 1, 2, 3].into_iter().enumerate() {
        let half = i as i32;
        fill_rect(page, RectPx::new(mid_x - half, track.y + 2 + dy, half * 2 + 1, 1), CHOOSE_MAP_SCROLL_THUMB);
        fill_rect(page, RectPx::new(mid_x - half, track.y + track.h - 3 - dy, half * 2 + 1, 1), CHOOSE_MAP_SCROLL_THUMB);
    }
}
