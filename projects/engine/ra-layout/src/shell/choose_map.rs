//! 选图页列表几何辅助（行高 / 滚动）。页面槽位权威为 `solve_choose_map`。

use super::*;

/// 选图页列表行高：`GAME.FNT` 字高 17 + 2。
pub const CHOOSE_MAP_LIST_ROW_H: i32 = 19;

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

/// 选图列表可视行矩形（`row_index` 为窗内行，非绝对下标）。
pub fn choose_map_list_row_rect(list: RectPx, row_index: usize, content_w: i32) -> RectPx {
    RectPx::new(list.x, list.y + (row_index as i32) * CHOOSE_MAP_LIST_ROW_H, content_w, CHOOSE_MAP_LIST_ROW_H)
}
