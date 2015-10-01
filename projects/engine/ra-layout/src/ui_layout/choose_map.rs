//! 选图页布局。

use super::*;
use crate::solve_choose_map;

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
    RectPx::new(
        list.x,
        list.y + (row_index as i32) * CHOOSE_MAP_LIST_ROW_H,
        content_w,
        CHOOSE_MAP_LIST_ROW_H,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseMapLayout {
    /// 共用壳层 chrome（背景 / 右栏）。选图页不画底条装饰。
    pub shell: MainMenuLayout,
    /// 右栏标题（`0x694` / `GUI:ChooseMap`）。
    pub title: RectPx,
    /// 右栏小地图预览（`0x468`）。
    pub map_preview: RectPx,
    /// 右栏地图名底板（`sdmpbtn`）。
    pub map_name_plate: RectPx,
    /// 「选择交战」说明（`GUI:SelectEngagement`）。
    pub label_engagement: RectPx,
    /// 「游戏类型」列标题（`GUI:GameType`）。
    pub label_game_type: RectPx,
    /// 「游戏地图」列标题（`GUI:GameMap`）。
    pub label_game_map: RectPx,
    /// 游戏类型列表（`0x6EB`）。
    pub game_type_list: RectPx,
    /// 地图列表（`0x553`）。
    pub map_list: RectPx,
    /// 底栏状态提示（`0x695`）。
    pub status_help: RectPx,
}

/// 选图页布局（800×600；面板 chrome 与 `0x6B` 控件同一次 snapshot）。
pub fn choose_map_layout(_viewport_w: u32, _viewport_h: u32) -> ChooseMapLayout {
    let snap = solve_choose_map();
    let shell = shell_rail_layout_from_snap(&snap, &CHOOSE_MAP_BUTTON_IDS);
    ChooseMapLayout {
        shell,
        title: rect_px_from_snapshot(&snap, "title"),
        map_preview: rect_px_from_snapshot(&snap, "map_preview"),
        map_name_plate: rect_px_from_snapshot(&snap, "map_name_plate"),
        label_engagement: rect_px_from_snapshot(&snap, "label_engagement"),
        label_game_type: rect_px_from_snapshot(&snap, "label_game_type"),
        label_game_map: rect_px_from_snapshot(&snap, "label_game_map"),
        game_type_list: rect_px_from_snapshot(&snap, "game_type_list"),
        map_list: rect_px_from_snapshot(&snap, "map_list"),
        status_help: rect_px_from_snapshot(&snap, "status_help"),
    }
}
