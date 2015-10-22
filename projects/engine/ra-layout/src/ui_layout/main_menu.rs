//! 主菜单 / 单人右栏布局。

use super::*;
use crate::{LayoutSnapshot, RightPanelChrome};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainMenuLayout {
    /// 合成画布（通常为 800×600；大窗时仍以此为内容基准）。
    pub canvas: RectPx,
    /// 背景放置原点（相对画布）。
    pub background: RectPx,
    /// 循环影片矩形（与父背景同区；解码后叠在背景上）。
    pub movie: RectPx,
    /// 右侧顶盖。
    pub panel_top: RectPx,
    /// 右侧平铺起点与单条尺寸（纵向重复）。
    pub panel_tile: RectPx,
    /// 平铺条数。
    pub panel_tile_count: i32,
    /// 右侧底盖。
    pub panel_bottom: RectPx,
    /// 底部装饰条。
    pub lower_strip: RectPx,
    /// 右侧顶盖内页标题（如「主選單」）。
    pub title: RectPx,
    /// 左下角悬停提示行。
    pub tooltip: RectPx,
    /// 六个主菜单按钮格；末项 Exit 贴底盖上沿。
    pub buttons: [RectPx; 6],
}

pub(super) fn layout_from_shell_page_snap(chrome: RightPanelChrome, snap: &LayoutSnapshot) -> MainMenuLayout {
    MainMenuLayout {
        canvas: RectPx::new(0, 0, chrome.shell_w as i32, chrome.shell_h as i32),
        background: rect_px_from_snapshot(snap, "background"),
        movie: rect_px_from_snapshot(snap, "movie"),
        panel_top: rect_px_from_snapshot(snap, "panel_top"),
        panel_tile: rect_px_from_snapshot(snap, "panel_tile"),
        panel_tile_count: chrome.tile_count(),
        panel_bottom: rect_px_from_snapshot(snap, "panel_bottom"),
        lower_strip: rect_px_from_snapshot(snap, "lower_strip"),
        title: rect_px_from_snapshot(snap, "title"),
        tooltip: rect_px_from_snapshot(snap, "tooltip"),
        buttons: [RectPx::new(0, 0, 0, 0); 6],
    }
}

pub(super) fn buttons_from_snap(snap: &LayoutSnapshot, ids: &[&str]) -> Vec<RectPx> {
    ids.iter()
        .map(|id| rect_px_from_snapshot(snap, id))
        .collect()
}

/// 由已有 shell-page snapshot 投影右栏壳层与按钮格（不足六格补空矩形）。
pub fn shell_rail_layout_from_snap(snap: &LayoutSnapshot, button_ids: &[&str]) -> MainMenuLayout {
    let chrome = RightPanelChrome::shell_defaults();
    let mut layout = layout_from_shell_page_snap(chrome, snap);
    let rail = buttons_from_snap(snap, button_ids);
    let mut buttons = [RectPx::new(0, 0, 0, 0); 6];
    for (i, rect) in rail.into_iter().enumerate().take(6) {
        buttons[i] = rect;
    }
    layout.buttons = buttons;
    layout
}

/// 按视口计算主菜单布局（内容落在 800×600 基准上；视口更大时由渲染相机居中）。
///
/// chrome 与右栏按钮均投影自同一次 `shell_page_layout_tree` 求解。
pub fn main_menu_layout(_viewport_w: u32, _viewport_h: u32) -> MainMenuLayout {
    let snap = crate::solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    shell_rail_layout_from_snap(&snap, &MAIN_MENU_BUTTON_IDS)
}

/// 单人页：前三连格 + 返回贴底盖（与主菜单 Exit 同锚点）。
pub fn single_player_layout(_viewport_w: u32, _viewport_h: u32) -> MainMenuLayout {
    let snap = crate::solve_shell_page(
        "single_player",
        &SINGLE_PLAYER_BUTTON_IDS[..3],
        Some(SINGLE_PLAYER_BUTTON_IDS[3]),
    );
    shell_rail_layout_from_snap(&snap, &SINGLE_PLAYER_BUTTON_IDS)
}
