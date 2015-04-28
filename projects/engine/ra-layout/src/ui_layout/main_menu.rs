//! 主菜单 / 单人 / 选项右栏布局。

use super::*;
use crate::{
    right_rail_buttons_layout_tree, LayoutEngine, RightPanelChrome, Viewport,
};

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

pub(super) fn right_rail_buttons(
    root_id: &str,
    stacked_ids: &[&str],
    bottom_id: Option<&str>,
) -> Vec<RectPx> {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: crate::shell_design_size(chrome),
            ..Viewport::default()
        },
        &right_rail_buttons_layout_tree(root_id, stacked_ids, bottom_id, chrome),
    );
    let mut out = Vec::with_capacity(stacked_ids.len() + bottom_id.is_some() as usize);
    for id in stacked_ids {
        out.push(rect_px_from_snapshot(&snap, id));
    }
    if let Some(id) = bottom_id {
        out.push(rect_px_from_snapshot(&snap, id));
    }
    out
}

/// 按视口计算主菜单布局（内容落在 800×600 基准上；视口更大时由渲染相机居中）。
///
/// 右侧底盖高度取「顶盖以下剩余高度按 42 整除后的余数」，Exit 贴底盖上沿一行
/// （对齐原版 0xE2 `OwnerDrawButtonBottomRow`），前五项占连续平铺格。
/// 右栏按钮几何来自 `right_rail_buttons_layout_tree`。
pub fn main_menu_layout(_viewport_w: u32, _viewport_h: u32) -> MainMenuLayout {
    let canvas = RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H);
    let panel_x = SHELL_BASE_W - RIGHT_PANEL_W;
    let panel_top = RectPx::new(panel_x, 0, RIGHT_PANEL_W, RIGHT_PANEL_TOP_H);
    let tile = RectPx::new(panel_x, RIGHT_PANEL_TOP_H, RIGHT_PANEL_W, RIGHT_PANEL_TILE_H);
    let remaining = (SHELL_BASE_H - RIGHT_PANEL_TOP_H).max(0);
    let tile_count = (remaining / RIGHT_PANEL_TILE_H).clamp(0, 9);
    let bottom_y = tile.y + tile_count * RIGHT_PANEL_TILE_H;
    let panel_bottom = RectPx::new(panel_x, bottom_y, RIGHT_PANEL_W, SHELL_BASE_H - bottom_y);
    // 原版 `ra2ts_l` 为 632×570；底条 `lwscrnl` 高 32 贴底，与影片下沿重叠 2px。
    let movie_w = panel_x;
    let movie_h = 570;
    let lower_strip = RectPx::new(0, SHELL_BASE_H - LOWER_STRIP_H, movie_w, LOWER_STRIP_H);
    let rail = right_rail_buttons(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    let buttons = [rail[0], rail[1], rail[2], rail[3], rail[4], rail[5]];
    // 原版标题：兼容宽 163×18，侧栏内 inset，顶盖下 y=9。
    let title = RectPx::new(panel_x + 3, 9, 163, 18);
    // 原版提示：底边上方 1px，左 inset 10，宽 455、高 20。
    let tooltip = RectPx::new(10, SHELL_BASE_H - 20 - 1, 455, 20);
    MainMenuLayout {
        canvas,
        // `mnscrnl` / 影片区：632×570，底边留给 `lwscrnl`。
        background: RectPx::new(0, 0, movie_w, movie_h),
        movie: RectPx::new(0, 0, movie_w, movie_h),
        panel_top,
        panel_tile: tile,
        panel_tile_count: tile_count,
        panel_bottom,
        lower_strip,
        title,
        tooltip,
        buttons,
    }
}

/// 单人页：前三连格 + 返回贴底盖（与主菜单 Exit 同锚点）。
pub fn single_player_layout(viewport_w: u32, viewport_h: u32) -> MainMenuLayout {
    let mut layout = main_menu_layout(viewport_w, viewport_h);
    let rail = right_rail_buttons(
        "single_player",
        &SINGLE_PLAYER_BUTTON_IDS[..3],
        Some(SINGLE_PLAYER_BUTTON_IDS[3]),
    );
    // 合成/命中仍读 `buttons[0..4]`；多出的两格不参与单人页。
    layout.buttons = [
        rail[0],
        rail[1],
        rail[2],
        rail[3],
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    layout
}

/// 选项页：接受 / 取消 / 主菜单贴底盖（左栏控件另由 `options_dialog` 绘制）。
pub fn options_layout(viewport_w: u32, viewport_h: u32) -> MainMenuLayout {
    let mut layout = main_menu_layout(viewport_w, viewport_h);
    let rail = right_rail_buttons(
        "options",
        &OPTIONS_BUTTON_IDS[..2],
        Some(OPTIONS_BUTTON_IDS[2]),
    );
    layout.buttons = [
        rail[0],
        rail[1],
        rail[2],
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    layout
}
