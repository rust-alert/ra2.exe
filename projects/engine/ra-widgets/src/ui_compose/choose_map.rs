//! 由一次性拆分自 `ui_compose.rs`。

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

/// 选图页列表行高：`GAME.FNT` 字高 17 + 2。
pub const CHOOSE_MAP_LIST_ROW_H: i32 = 19;

/// 溢出时列表内容区右侧留给滚动条的宽度（`1*2+0x12`）。
pub const CHOOSE_MAP_SCROLL_W: i32 = 20;

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
    map_list_scroll: usize,
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

    fill_rect(&mut page, layout.game_type_list, CHOOSE_MAP_LIST_BG);
    stroke_rect(&mut page, layout.game_type_list, CHOOSE_MAP_LIST_BORDER);
    fill_rect(&mut page, layout.map_list, CHOOSE_MAP_LIST_BG);
    stroke_rect(&mut page, layout.map_list, CHOOSE_MAP_LIST_BORDER);

    let visible_modes = (layout.game_type_list.h / CHOOSE_MAP_LIST_ROW_H).max(0) as usize;
    for (i, name) in mode_names.iter().take(visible_modes).enumerate() {
        let row = RectPx::new(
            layout.game_type_list.x,
            layout.game_type_list.y + (i as i32) * CHOOSE_MAP_LIST_ROW_H,
            layout.game_type_list.w,
            CHOOSE_MAP_LIST_ROW_H,
        );
        if Some(i) == selected_mode_index {
            fill_rect(&mut page, row, CHOOSE_MAP_LIST_SELECTED);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(&mut page, fnt, name, row.x + 2, row.y, row.w - 4, row.h, CHOOSE_MAP_LIST_TEXT);
        }
    }

    let visible_rows = choose_map_visible_rows(layout.map_list.h);
    let scroll = clamp_map_list_scroll(map_list_scroll, map_names.len(), visible_rows);
    let map_overflow = map_names.len() > visible_rows && visible_rows > 0;
    let map_content_w = if map_overflow {
        (layout.map_list.w - CHOOSE_MAP_SCROLL_W).max(8)
    } else {
        layout.map_list.w
    };
    for (row_i, name) in map_names.iter().skip(scroll).take(visible_rows).enumerate() {
        let abs_i = scroll + row_i;
        let row = RectPx::new(
            layout.map_list.x,
            layout.map_list.y + (row_i as i32) * CHOOSE_MAP_LIST_ROW_H,
            map_content_w,
            CHOOSE_MAP_LIST_ROW_H,
        );
        if Some(abs_i) == selected_map_index {
            fill_rect(&mut page, row, CHOOSE_MAP_LIST_SELECTED);
        }
        if let Some(fnt) = fnt {
            blit_caption_top_left_clipped(
                &mut page,
                fnt,
                name,
                row.x + 2,
                row.y,
                (row.w - 4).max(8),
                row.h,
                CHOOSE_MAP_LIST_TEXT,
            );
        }
    }

    paint_choose_map_scrollbar(&mut page, layout.map_list, map_names.len(), visible_rows, scroll);

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
        fill_rect(
            page,
            RectPx::new(mid_x - half, track.y + track.h - 3 - dy, half * 2 + 1, 1),
            CHOOSE_MAP_SCROLL_THUMB,
        );
    }
}
