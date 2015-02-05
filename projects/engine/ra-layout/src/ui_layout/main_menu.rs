//! 主菜单 / 单人 / 选项右栏布局。

use super::*;


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

pub(super) fn button_cell(panel_x: i32, y: i32) -> RectPx {
    let x = panel_x + (RIGHT_PANEL_W - BUTTON_CELL_W);
    RectPx::new(x, y, BUTTON_CELL_W, BUTTON_CELL_H)
}

/// 按视口计算主菜单布局（内容落在 800×600 基准上；视口更大时由渲染相机居中）。
///
/// 右侧底盖高度取「顶盖以下剩余高度按 42 整除后的余数」，Exit 贴底盖上沿一行
/// （对齐原版 0xE2 `OwnerDrawButtonBottomRow`），前五项占连续平铺格。
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
    let exit_y = panel_bottom.y - BUTTON_CELL_H;
    let buttons = [
        button_cell(panel_x, tile.y),
        button_cell(panel_x, tile.y + BUTTON_CELL_H),
        button_cell(panel_x, tile.y + 2 * BUTTON_CELL_H),
        button_cell(panel_x, tile.y + 3 * BUTTON_CELL_H),
        button_cell(panel_x, tile.y + 4 * BUTTON_CELL_H),
        button_cell(panel_x, exit_y),
    ];
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

pub(super) fn four_stack_plus_exit(panel_x: i32, tile_y: i32, exit_y: i32) -> [RectPx; 4] {
    [
        button_cell(panel_x, tile_y),
        button_cell(panel_x, tile_y + BUTTON_CELL_H),
        button_cell(panel_x, tile_y + 2 * BUTTON_CELL_H),
        button_cell(panel_x, exit_y),
    ]
}

/// 单人页：前三连格 + 返回贴底盖（与主菜单 Exit 同锚点）。
pub fn single_player_layout(viewport_w: u32, viewport_h: u32) -> MainMenuLayout {
    let mut layout = main_menu_layout(viewport_w, viewport_h);
    let exit_y = layout.panel_bottom.y - BUTTON_CELL_H;
    let four = four_stack_plus_exit(layout.panel_top.x, layout.panel_tile.y, exit_y);
    // 合成/命中仍读 `buttons[0..4]`；多出的两格不参与单人页。
    layout.buttons = [four[0], four[1], four[2], four[3], RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0)];
    layout
}

/// 选项页：接受 / 取消 / 主菜单贴底盖（左栏控件另由 `options_dialog` 绘制）。
pub fn options_layout(viewport_w: u32, viewport_h: u32) -> MainMenuLayout {
    let mut layout = main_menu_layout(viewport_w, viewport_h);
    let exit_y = layout.panel_bottom.y - BUTTON_CELL_H;
    let panel_x = layout.panel_top.x;
    let tile_y = layout.panel_tile.y;
    layout.buttons = [
        button_cell(panel_x, tile_y),
        button_cell(panel_x, tile_y + BUTTON_CELL_H),
        button_cell(panel_x, exit_y),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    layout
}
