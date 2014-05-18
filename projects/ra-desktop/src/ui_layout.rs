//! 主菜单壳层像素布局（800×600 基准，大窗居中）。
//!
//! 几何服务合成与命中；大窗时用与渲染相同的 [`ra_renderer::ViewCamera::fit`] 映射。

use ra_renderer::ViewCamera;

/// 壳层设计宽。
pub const SHELL_BASE_W: i32 = 800;
/// 壳层设计高。
pub const SHELL_BASE_H: i32 = 600;
/// 右侧面板宽。
pub const RIGHT_PANEL_W: i32 = 168;
/// 右侧顶盖高（`sdtp`）。
pub const RIGHT_PANEL_TOP_H: i32 = 199;
/// 右侧平铺条高（`sdbtnbkgd`）。
pub const RIGHT_PANEL_TILE_H: i32 = 42;
/// 按钮格宽（窄列）。
pub const BUTTON_CELL_W: i32 = 156;
/// 按钮格高。
pub const BUTTON_CELL_H: i32 = 42;

/// 主菜单按钮入口 id（与 [`crate::ui_slots`] 顺序一致）。
pub const MAIN_MENU_BUTTON_IDS: [&str; 4] = ["single_player", "network", "options", "exit"];

/// 单人页按钮入口 id（与 [`crate::ui_slots`] 顺序一致）。
pub const SINGLE_PLAYER_BUTTON_IDS: [&str; 4] = ["campaign", "skirmish", "training", "back"];

/// 遭遇战大厅右侧按钮入口 id（与 `ui_slots` 顺序一致）。
pub const SKIRMISH_LOBBY_BUTTON_IDS: [&str; 4] = ["side", "difficulty", "start", "back"];

/// 大厅地图列表最多可见行。
pub const LOBBY_MAP_ROW_MAX: i32 = 6;
/// 大厅地图列表行高（像素）。
pub const LOBBY_MAP_ROW_H: i32 = 28;
/// 大厅地图列表行间距。
pub const LOBBY_MAP_ROW_GAP: i32 = 6;

/// 轴对齐矩形（像素）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectPx {
    /// 左。
    pub x: i32,
    /// 上。
    pub y: i32,
    /// 宽。
    pub w: i32,
    /// 高。
    pub h: i32,
}

impl RectPx {
    /// 构造。
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// 是否包含点（像素，半开区间右下）。
    pub fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }
}

/// 主菜单一帧布局。
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
    /// 四个主菜单按钮格（单人 / 网络 / 选项 / 退出）。
    pub buttons: [RectPx; 4],
}

fn button_cell(panel_x: i32, tile_y: i32, row: i32) -> RectPx {
    let x = panel_x + (RIGHT_PANEL_W - BUTTON_CELL_W);
    let y = tile_y + row * BUTTON_CELL_H;
    RectPx::new(x, y, BUTTON_CELL_W, BUTTON_CELL_H)
}

/// 与 UI 页上传后相同的 fit 相机（内容 800×600 → 窗口）。
pub fn shell_fit_camera(win_w: u32, win_h: u32) -> ViewCamera {
    ViewCamera::fit(SHELL_BASE_W as u32, SHELL_BASE_H as u32, win_w.max(1), win_h.max(1))
}

/// 窗口像素 → 壳层内容像素（与 [`shell_fit_camera`] 一致）。
pub fn window_to_shell_px(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> (i32, i32) {
    let cam = shell_fit_camera(win_w.max(1.0) as u32, win_h.max(1.0) as u32);
    let (wx, wy) = cam.screen_to_world(cursor_x as f32, cursor_y as f32, win_w.max(1.0) as f32, win_h.max(1.0) as f32);
    (wx.floor() as i32, wy.floor() as i32)
}

/// 按视口计算主菜单布局（内容落在 800×600 基准上；视口更大时由渲染相机居中）。
pub fn main_menu_layout(_viewport_w: u32, _viewport_h: u32) -> MainMenuLayout {
    let canvas = RectPx::new(0, 0, SHELL_BASE_W, SHELL_BASE_H);
    let panel_x = SHELL_BASE_W - RIGHT_PANEL_W;
    let panel_top = RectPx::new(panel_x, 0, RIGHT_PANEL_W, RIGHT_PANEL_TOP_H);
    let tile = RectPx::new(panel_x, RIGHT_PANEL_TOP_H, RIGHT_PANEL_W, RIGHT_PANEL_TILE_H);
    // 底盖高度按零售 `sdbtm` 画布 65 近似。
    let bottom_h = 65;
    let remaining = (SHELL_BASE_H - RIGHT_PANEL_TOP_H - bottom_h).max(0);
    let tile_count = (remaining / RIGHT_PANEL_TILE_H).clamp(0, 9);
    let bottom_y = tile.y + tile_count * RIGHT_PANEL_TILE_H;
    let panel_bottom = RectPx::new(panel_x, bottom_y, RIGHT_PANEL_W, SHELL_BASE_H - bottom_y);
    // `lwscrnl` 画布高 32；贴在内容区底边。
    let lower_strip = RectPx::new(0, SHELL_BASE_H - 32, panel_x, 32);
    // 按钮落在平铺列上：从顶盖下第一格起连续四格。
    let buttons = [
        button_cell(panel_x, tile.y, 0),
        button_cell(panel_x, tile.y, 1),
        button_cell(panel_x, tile.y, 2),
        button_cell(panel_x, tile.y, 3),
    ];
    MainMenuLayout {
        canvas,
        // 父背景与影片区同左上；`mnscrnl` 约 632×568，不铺满 800 宽。
        background: RectPx::new(0, 0, 632, 568),
        movie: RectPx::new(0, 0, 632, 568),
        panel_top,
        panel_tile: tile,
        panel_tile_count: tile_count,
        panel_bottom,
        lower_strip,
        buttons,
    }
}

/// 单人页布局：当前与主菜单共用右侧壳层几何（按钮 id 不同）。
pub fn single_player_layout(viewport_w: u32, viewport_h: u32) -> MainMenuLayout {
    main_menu_layout(viewport_w, viewport_h)
}

/// 遭遇战大厅布局：右侧按钮格与主菜单同几何；`movie` 区作地图预览占位。
pub fn skirmish_lobby_layout(viewport_w: u32, viewport_h: u32) -> MainMenuLayout {
    main_menu_layout(viewport_w, viewport_h)
}

/// 大厅地图列表第 `index` 行的像素矩形（内容坐标）。
pub fn skirmish_map_row_rect(layout: &MainMenuLayout, index: usize) -> RectPx {
    let list_x = 24;
    let list_w = (layout.movie.w - 48).max(1);
    let y = layout.movie.y + 48 + (index as i32) * (LOBBY_MAP_ROW_H + LOBBY_MAP_ROW_GAP);
    RectPx::new(list_x, y, list_w, LOBBY_MAP_ROW_H)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_panel_sits_on_right_edge() {
        let layout = main_menu_layout(1024, 768);
        assert_eq!(layout.canvas.w, 800);
        assert_eq!(layout.panel_top.x + layout.panel_top.w, 800);
        assert!(layout.panel_tile_count > 0);
        assert_eq!(layout.buttons[0].w, BUTTON_CELL_W);
        assert_eq!(layout.buttons[0].y, RIGHT_PANEL_TOP_H);
        assert_eq!(layout.buttons[3].y, RIGHT_PANEL_TOP_H + 3 * BUTTON_CELL_H);
        assert_eq!(MAIN_MENU_BUTTON_IDS.len(), layout.buttons.len());
    }

    #[test]
    fn skirmish_lobby_reuses_right_panel() {
        let layout = skirmish_lobby_layout(1024, 768);
        assert_eq!(SKIRMISH_LOBBY_BUTTON_IDS.len(), layout.buttons.len());
        assert_eq!(layout.buttons[0].y, RIGHT_PANEL_TOP_H);
        let row0 = skirmish_map_row_rect(&layout, 0);
        assert!(row0.w > 0);
        assert!(row0.y >= layout.movie.y);
    }

    fn window_center_maps_near_shell_center_when_fitted() {
        let (x, y) = window_to_shell_px(512.0, 384.0, 1024.0, 768.0);
        assert!((x - 400).abs() <= 2);
        assert!((y - 300).abs() <= 2);
    }
}
