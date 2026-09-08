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
/// 底部装饰条高（`lwscrnl`）。
pub const LOWER_STRIP_H: i32 = 32;

/// 主菜单按钮入口 id（与 [`crate::ui_slots`] 顺序一致）。
///
/// 顺序对齐原版 0xE2：前五项连续平铺格，末项 Exit 贴底盖。
pub const MAIN_MENU_BUTTON_IDS: [&str; 6] = [
    "single_player",
    "ww_online",
    "network",
    "movies",
    "options",
    "exit",
];

/// 单人页按钮入口 id（与 [`crate::ui_slots`] 顺序一致）。
pub const SINGLE_PLAYER_BUTTON_IDS: [&str; 4] = ["campaign", "skirmish", "training", "back"];

/// 遭遇战大厅右侧按钮入口 id（与 `ui_slots` 顺序一致）。
pub const SKIRMISH_LOBBY_BUTTON_IDS: [&str; 4] = ["side", "difficulty", "start", "back"];

/// 选项页按钮入口 id（与 [`crate::ui_slots`] 顺序一致）。
pub const OPTIONS_BUTTON_IDS: [&str; 3] = ["accept", "cancel", "main_menu"];

/// 退出确认对话框按钮 id。
pub const EXIT_CONFIRM_BUTTON_IDS: [&str; 2] = ["ok", "cancel"];

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
    /// 右侧顶盖内页标题（如「主選單」）。
    pub title: RectPx,
    /// 左下角悬停提示行。
    pub tooltip: RectPx,
    /// 六个主菜单按钮格；末项 Exit 贴底盖上沿。
    pub buttons: [RectPx; 6],
}

fn button_cell(panel_x: i32, y: i32) -> RectPx {
    let x = panel_x + (RIGHT_PANEL_W - BUTTON_CELL_W);
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
///
/// 右侧底盖高度取「顶盖以下剩余高度按 42 整除后的余数」，Exit 贴底盖上沿一行
///（对齐原版 0xE2 `OwnerDrawButtonBottomRow`），前五项占连续平铺格。
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

fn four_stack_plus_exit(panel_x: i32, tile_y: i32, exit_y: i32) -> [RectPx; 4] {
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
    layout.buttons = [
        four[0],
        four[1],
        four[2],
        four[3],
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    layout
}

/// 遭遇战大厅专用布局（壳层 chrome + 左侧列表/预览分区）。
///
/// 不再复用单人页布局函数；右侧四钮几何可与壳层惯例相近，但左侧分区独立。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishLobbyLayout {
    /// 共用壳层 chrome（背景 / 右栏 / 底条）。
    pub shell: MainMenuLayout,
    /// 地图名列表区（左上）。
    pub map_list: RectPx,
    /// 地图预览区（列表下方）。
    pub map_preview: RectPx,
}

/// 遭遇战大厅布局（800×600 内容坐标）。
pub fn skirmish_lobby_layout(viewport_w: u32, viewport_h: u32) -> SkirmishLobbyLayout {
    let shell = main_menu_layout(viewport_w, viewport_h);
    let exit_y = shell.panel_bottom.y - BUTTON_CELL_H;
    let panel_x = shell.panel_top.x;
    let tile_y = shell.panel_tile.y;
    let mut shell = shell;
    // 右栏：阵营 / 难度 / 开始 连格，返回贴底盖。
    shell.buttons = [
        button_cell(panel_x, tile_y),
        button_cell(panel_x, tile_y + BUTTON_CELL_H),
        button_cell(panel_x, tile_y + 2 * BUTTON_CELL_H),
        button_cell(panel_x, exit_y),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    let content_w = (shell.movie.w - 32).max(1);
    let list_h = LOBBY_MAP_ROW_MAX * LOBBY_MAP_ROW_H + (LOBBY_MAP_ROW_MAX - 1) * LOBBY_MAP_ROW_GAP + 16;
    let map_list = RectPx::new(shell.movie.x + 16, shell.movie.y + 16, content_w, list_h);
    let preview_y = map_list.y + map_list.h + 12;
    let preview_h = (shell.movie.y + shell.movie.h - preview_y - 16).max(80);
    let map_preview = RectPx::new(shell.movie.x + 16, preview_y, content_w, preview_h);
    SkirmishLobbyLayout {
        shell,
        map_list,
        map_preview,
    }
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

/// 大厅地图列表第 `index` 行的像素矩形（相对大厅布局）。
pub fn skirmish_map_row_rect(layout: &SkirmishLobbyLayout, index: usize) -> RectPx {
    let list_x = layout.map_list.x + 8;
    let list_w = (layout.map_list.w - 16).max(1);
    let y = layout.map_list.y + 8 + (index as i32) * (LOBBY_MAP_ROW_H + LOBBY_MAP_ROW_GAP);
    RectPx::new(list_x, y, list_w, LOBBY_MAP_ROW_H)
}

/// 退出确认居中框几何（800×600 内容坐标）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitConfirmLayout {
    /// 对话框底板。
    pub dialog: RectPx,
    /// 提示文案区。
    pub prompt: RectPx,
    /// 确定 / 取消。
    pub buttons: [RectPx; 2],
}

/// 退出确认布局（相对壳层画布居中）。
pub fn exit_confirm_layout(_viewport_w: u32, _viewport_h: u32) -> ExitConfirmLayout {
    let dialog = RectPx::new(180, 180, 440, 200);
    let prompt = RectPx::new(dialog.x + 24, dialog.y + 28, dialog.w - 48, 64);
    let btn_y = dialog.y + dialog.h - BUTTON_CELL_H - 24;
    let gap = 24;
    let total_w = BUTTON_CELL_W * 2 + gap;
    let btn_x0 = dialog.x + (dialog.w - total_w) / 2;
    ExitConfirmLayout {
        dialog,
        prompt,
        buttons: [
            RectPx::new(btn_x0, btn_y, BUTTON_CELL_W, BUTTON_CELL_H),
            RectPx::new(btn_x0 + BUTTON_CELL_W + gap, btn_y, BUTTON_CELL_W, BUTTON_CELL_H),
        ],
    }
}
