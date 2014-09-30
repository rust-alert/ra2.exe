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
/// WARNING 屏动画（`sdwrnanm`）画布宽。
pub const SDWRNANM_W: i32 = 92;
/// WARNING 屏动画（`sdwrnanm`）画布高。
pub const SDWRNANM_H: i32 = 53;
/// `sdwrnanm` 相对 `sdtp` 左上角的 X 偏移。
pub const SDWRNANM_OFFSET_X: i32 = 38;
/// `sdwrnanm` 相对 `sdtp` 左上角的 Y 偏移。
pub const SDWRNANM_OFFSET_Y: i32 = 48;
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
///
/// 顺序：新战役 → 载入 → 遭遇战 → 主菜单（贴底盖）。
pub const SINGLE_PLAYER_BUTTON_IDS: [&str; 4] = ["campaign", "load", "skirmish", "back"];

/// 战役页右栏按钮入口 id（与 [`crate::ui_slots`] 顺序一致）。
///
/// 顺序对齐对话框 `0x94`：载入 → 上一页（贴底）。
pub const CAMPAIGN_BUTTON_IDS: [&str; 2] = ["load", "back"];

/// 战役三侧入口 id（盟军 / 新兵训练营 / 苏军；`battle.ini` 的 ALL1 / TUT1 / SOV1）。
pub const CAMPAIGN_SIDE_IDS: [&str; 3] = ["allied", "tutorial", "soviet"];

/// 遭遇战大厅右侧按钮入口 id（与 `ui_slots` 顺序一致）。
///
/// 顺序对齐原版 0x102：开始游戏 → 选图（自订战役）→ 上一页（贴底）。
pub const SKIRMISH_LOBBY_BUTTON_IDS: [&str; 3] = ["start", "choose_map", "back"];

/// 选图页右侧按钮入口 id（与 `ui_slots` 顺序一致）。
///
/// 顺序对齐安装 `game.exe` 的 `RT_DIALOG` id=`107`（`0x6B`）：
/// 使用地图 → 创建随机地图 → 取消（贴底）。
pub const CHOOSE_MAP_BUTTON_IDS: [&str; 3] = ["use_map", "create_random", "cancel"];

/// 选项页按钮入口 id（与 [`crate::ui_slots`] 顺序一致）。
pub const OPTIONS_BUTTON_IDS: [&str; 3] = ["accept", "cancel", "main_menu"];

/// 退出确认对话框按钮 id。
pub const EXIT_CONFIRM_BUTTON_IDS: [&str; 2] = ["ok", "cancel"];

/// 遭遇战玩家行数（本地 + AI）。
pub const SKIRMISH_ROW_COUNT: usize = 8;
/// 遭遇战 AI 行数（不含本地玩家）。
pub const SKIRMISH_AI_ROW_COUNT: usize = 7;
/// 下拉面高度（像素）。
pub const SKIRMISH_COMBO_FACE_H: i32 = 24;
/// 勾选图标宽。
pub const SKIRMISH_CHECK_W: i32 = 18;
/// 勾选图标高。
pub const SKIRMISH_CHECK_H: i32 = 18;

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

fn dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx {
    // MS Sans Serif 8pt：x×6/4、y×13/8，四舍五入。
    fn mul_div_round(n: i32, numer: i32, denom: i32) -> i32 {
        let value = n * numer;
        if value >= 0 {
            (value + denom / 2) / denom
        } else {
            (value - denom / 2) / denom
        }
    }
    RectPx::new(
        mul_div_round(x, 6, 4),
        mul_div_round(y, 13, 8),
        mul_div_round(w, 6, 4),
        mul_div_round(h, 13, 8),
    )
}

fn skirmish_snap_button(source: RectPx, panel_tile_y: i32) -> RectPx {
    // 偏置截断：相对 `sdbtnbkgd` 列顶，按 42px 格吸附（壳层 chrome，非截图估）。
    let tile_h = RIGHT_PANEL_TILE_H.max(1);
    let tile_index = ((source.y - panel_tile_y + tile_h / 2) / tile_h).max(0);
    button_cell(SHELL_BASE_W - RIGHT_PANEL_W, panel_tile_y + tile_index * tile_h)
}

fn skirmish_right_anchor(base: RectPx) -> RectPx {
    // 右栏静态/预览：相对 `RIGHT_PANEL_W` 水平居中锚到右缘。
    let inset = (RIGHT_PANEL_W - base.w) / 2;
    RectPx::new(SHELL_BASE_W - base.w - inset, base.y, base.w, base.h)
}

fn combo_face(dlu: RectPx) -> RectPx {
    RectPx::new(dlu.x, dlu.y, dlu.w, SKIRMISH_COMBO_FACE_H)
}

/// 遭遇战大厅专用布局（左：玩家行 + 选项；右：预览 + 开始/选图/返回）。
///
/// 控件 DLU 取自安装 `game.exe` 的 `RT_DIALOG` id=`258`（`0x102`）模板；
/// 右栏 owner-draw 钮再按壳层 42px 格吸附。禁止按截图像素估坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishLobbyLayout {
    /// 共用壳层 chrome（背景 / 右栏）。遭遇战不画底条装饰。
    pub shell: MainMenuLayout,
    /// 右栏小地图预览（`0x468`）。
    pub map_preview: RectPx,
    /// 右栏标题（`0x694` / `GUI:SkirmishGame`）。
    pub title: RectPx,
    /// 右栏游戏类型（`0x6EC`）。
    pub game_type: RectPx,
    /// 右栏地图名（`0x5A8`）。
    pub map_label: RectPx,
    /// 本地玩家名（`0x6A0`）。
    pub player_name: RectPx,
    /// 各行阵营旗（`0x6DA`..`0x6E1`）。
    pub flags: [RectPx; SKIRMISH_ROW_COUNT],
    /// 各行国家下拉面。
    pub side_faces: [RectPx; SKIRMISH_ROW_COUNT],
    /// 各行颜色下拉面。
    pub color_faces: [RectPx; SKIRMISH_ROW_COUNT],
    /// AI 难度/类型下拉面（行 1..=7）。
    pub ai_faces: [RectPx; SKIRMISH_AI_ROW_COUNT],
    /// 勾选：`0x54E` / `0x693` / `0x696` / `0x69A` / `0x69D`。
    pub checkboxes: [RectPx; 5],
    /// 游戏速度滑条（`0x529`）。
    pub track_speed: RectPx,
    /// 资金滑条（`0x511`）。
    pub track_credits: RectPx,
    /// 部队数滑条（`0x50C`）。
    pub track_units: RectPx,
    /// 速度说明（`0x699` / `GUI:GameSpeed`）。
    pub label_speed: RectPx,
    /// 资金说明（`0x69B` / `GUI:Credits`）。
    pub label_credits: RectPx,
    /// 部队数说明（`0x69C` / `GUI:UnitCount`）。
    pub label_units: RectPx,
    /// 底栏状态提示（`0x695`）。
    pub status_help: RectPx,
}

/// 遭遇战大厅布局（800×600 内容坐标；DLU→px 用 MS Sans Serif 8pt）。
pub fn skirmish_lobby_layout(viewport_w: u32, viewport_h: u32) -> SkirmishLobbyLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    // 遭遇战无 `lwscrnl` 底条；右栏三钮：开始 / 选图 / 返回（贴底盖格）。
    shell.lower_strip = RectPx::new(0, 0, 0, 0);
    let start = skirmish_snap_button(dlu_rect(318, 149, 108, 23), shell.panel_tile.y);
    let choose = skirmish_snap_button(dlu_rect(318, 176, 108, 23), shell.panel_tile.y);
    // 返回：壳层贴底盖上沿一行（owner-draw 底行惯例），不用对话框里偏上的 `0x5C0` y。
    let back = button_cell(shell.panel_top.x, shell.panel_bottom.y - BUTTON_CELL_H);
    shell.buttons = [
        start,
        choose,
        back,
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];

    // 行 y DLU：本地 11，其后每行 +16（与模板旗标/下拉一致）。
    let row_y = |i: usize| 11 + (i as i32) * 16;
    let mut flags = [RectPx::new(0, 0, 0, 0); SKIRMISH_ROW_COUNT];
    let mut side_faces = [RectPx::new(0, 0, 0, 0); SKIRMISH_ROW_COUNT];
    let mut color_faces = [RectPx::new(0, 0, 0, 0); SKIRMISH_ROW_COUNT];
    let mut ai_faces = [RectPx::new(0, 0, 0, 0); SKIRMISH_AI_ROW_COUNT];
    for i in 0..SKIRMISH_ROW_COUNT {
        let y = row_y(i);
        flags[i] = dlu_rect(143, y, 32, 12);
        side_faces[i] = combo_face(dlu_rect(180, y, 78, 74));
        color_faces[i] = combo_face(dlu_rect(264, y, 35, 73));
    }
    for i in 0..SKIRMISH_AI_ROW_COUNT {
        let y = row_y(i + 1);
        ai_faces[i] = combo_face(dlu_rect(35, y, 100, 74));
    }

    SkirmishLobbyLayout {
        shell,
        map_preview: skirmish_right_anchor(dlu_rect(324, 23, 96, 69)),
        title: skirmish_right_anchor(dlu_rect(318, 1, 108, 10)),
        game_type: skirmish_right_anchor(dlu_rect(327, 103, 90, 10)),
        map_label: skirmish_right_anchor(dlu_rect(327, 116, 90, 20)),
        player_name: dlu_rect(35, 11, 100, 12),
        flags,
        side_faces,
        color_faces,
        ai_faces,
        checkboxes: [
            dlu_rect(35, 145, 100, 10),
            dlu_rect(35, 162, 100, 10),
            dlu_rect(35, 179, 100, 10),
            dlu_rect(35, 197, 103, 10),
            dlu_rect(146, 196, 166, 11),
        ],
        track_speed: dlu_rect(214, 145, 85, 13),
        track_credits: dlu_rect(214, 162, 85, 13),
        track_units: dlu_rect(214, 179, 85, 13),
        label_speed: dlu_rect(146, 145, 60, 10),
        label_credits: dlu_rect(146, 162, 60, 10),
        label_units: dlu_rect(146, 179, 60, 10),
        status_help: dlu_rect(10, 282, 303, 12),
    }
}

/// 战役选边页布局（左：三侧图 + 难度；右：载入 / 返回）。
///
/// 控件 DLU 取自安装 `game.exe` 的 `RT_DIALOG` id=`148`（`0x94`）模板；
/// 右栏 owner-draw 钮再按壳层 42px 格吸附。禁止按截图像素估坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignLayout {
    /// 共用壳层 chrome（背景 / 右栏 / 底条）。
    pub shell: MainMenuLayout,
    /// 右栏标题（与主菜单壳层 `title` 同格；CSF `GUI:CampaignMenu`）。
    pub title: RectPx,
    /// 盟军侧图（`0x6EA` / `fsalg.shp`）。
    pub allied: RectPx,
    /// 新兵训练营侧图（`0x6EB` / `fsbclg.shp`）。
    pub tutorial: RectPx,
    /// 苏军侧图（`0x6EC` / `fsslg.shp`）。
    pub soviet: RectPx,
    /// 难度标签（`0x71E` / `GUI:Difficulty`）。
    pub difficulty_label: RectPx,
    /// 难度当前值（`0x670`）。
    pub difficulty_value: RectPx,
    /// 难度滑条（`0x50F`）。
    pub difficulty_track: RectPx,
    /// 底栏状态提示（与主菜单壳层 `tooltip` 同格）。
    pub status_help: RectPx,
}

/// 战役页布局（800×600 内容坐标；DLU→px 用 MS Sans Serif 8pt）。
pub fn campaign_layout(viewport_w: u32, viewport_h: u32) -> CampaignLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    // 载入：`0x40E` (318,122,108,23)；返回贴底盖（不用模板偏上的 `0x686` y）。
    let load = skirmish_snap_button(dlu_rect(318, 122, 108, 23), shell.panel_tile.y);
    let back = button_cell(shell.panel_top.x, shell.panel_bottom.y - BUTTON_CELL_H);
    shell.buttons = [
        load,
        back,
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    CampaignLayout {
        shell,
        // 右栏标题 / 底栏提示跟主菜单壳层 chrome 同格（`y=9` / 贴底），
        // 不用对话框 DLU `(318,1)` / `(8,282)`：那会把标题贴顶、提示悬在左区中下部。
        title: shell.title,
        allied: dlu_rect(16, 10, 284, 71),
        tutorial: dlu_rect(41, 85, 232, 56),
        soviet: dlu_rect(52, 145, 212, 71),
        difficulty_label: dlu_rect(90, 234, 75, 12),
        difficulty_value: dlu_rect(155, 234, 75, 12),
        difficulty_track: dlu_rect(90, 250, 140, 13),
        status_help: shell.tooltip,
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

/// 退出确认 MessageBox 面板宽（安装内 `pudlgbgn.shp` 画布；非 DLU 四舍五入的 450）。
pub const EXIT_CONFIRM_DIALOG_W: i32 = 451;
/// 退出确认 MessageBox 面板高（安装内 `pudlgbgn.shp` 画布；非 DLU 四舍五入的 325）。
pub const EXIT_CONFIRM_DIALOG_H: i32 = 326;
/// MessageBox 按钮艺术宽（安装内 `mnbttn.shp` 画布）。
pub const EXIT_CONFIRM_BUTTON_W: i32 = 126;
/// MessageBox 按钮艺术高（安装内 `mnbttn.shp` 画布）。
pub const EXIT_CONFIRM_BUTTON_H: i32 = 25;

/// 退出确认居中框几何（800×600 内容坐标）。
///
/// 面板尺寸取自 `pudlgbgn` 画布。子控件仍按模板 0x120 的 DLU：
/// 正文 `0x5B0` `(40,40,220,50)`，确定 `0x5AE` `(207,135,83,15)`，取消 id=2 `(207,175,83,15)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitConfirmLayout {
    /// 对话框底板（`pudlgbgn`）。
    pub dialog: RectPx,
    /// 提示文案区（左上锚点，非居中）。
    pub prompt: RectPx,
    /// 确定 / 取消（DLU 控件格；`mnbttn` 自左上贴齐，可溢出 1px）。
    pub buttons: [RectPx; 2],
}

fn modal_child(dialog: RectPx, local: RectPx) -> RectPx {
    RectPx::new(dialog.x + local.x, dialog.y + local.y, local.w, local.h)
}

/// 退出确认布局（相对壳层画布居中；底下仍是主菜单右栏）。
pub fn exit_confirm_layout(_viewport_w: u32, _viewport_h: u32) -> ExitConfirmLayout {
    let dialog = RectPx::new(
        (((SHELL_BASE_W - EXIT_CONFIRM_DIALOG_W) + 1) / 2).max(0),
        (((SHELL_BASE_H - EXIT_CONFIRM_DIALOG_H) + 1) / 2).max(0),
        EXIT_CONFIRM_DIALOG_W,
        EXIT_CONFIRM_DIALOG_H,
    );
    ExitConfirmLayout {
        dialog,
        prompt: modal_child(dialog, dlu_rect(40, 40, 220, 50)),
        // 控件原点取 DLU，宽高取 `mnbttn` 画布（126×25），避免 125×24 格内居中错位。
        buttons: [
            {
                let r = modal_child(dialog, dlu_rect(207, 135, 83, 15));
                RectPx::new(r.x, r.y, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H)
            },
            {
                let r = modal_child(dialog, dlu_rect(207, 175, 83, 15));
                RectPx::new(r.x, r.y, EXIT_CONFIRM_BUTTON_W, EXIT_CONFIRM_BUTTON_H)
            },
        ],
    }
}

/// 选图页布局（左：模式/地图列表；右：预览 + 使用地图 / 随机 / 取消）。
///
/// 控件 DLU 取自安装 `game.exe` 的 `RT_DIALOG` id=`107`（`0x6B`）模板；
/// 右栏 owner-draw 钮再按壳层 42px 格吸附。禁止按截图像素估坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChooseMapLayout {
    /// 共用壳层 chrome（背景 / 右栏）。选图页不画底条装饰。
    pub shell: MainMenuLayout,
    /// 右栏标题（`0x694` / `GUI:ChooseMap`）。
    pub title: RectPx,
    /// 右栏小地图预览（`0x468`）。
    pub map_preview: RectPx,
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

/// 选图页布局（800×600 内容坐标；DLU→px 用 MS Sans Serif 8pt）。
pub fn choose_map_layout(viewport_w: u32, viewport_h: u32) -> ChooseMapLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    shell.lower_strip = RectPx::new(0, 0, 0, 0);
    let use_map = skirmish_snap_button(dlu_rect(318, 122, 108, 23), shell.panel_tile.y);
    let create_random = skirmish_snap_button(dlu_rect(318, 149, 108, 23), shell.panel_tile.y);
    let cancel = button_cell(shell.panel_top.x, shell.panel_bottom.y - BUTTON_CELL_H);
    shell.buttons = [
        use_map,
        create_random,
        cancel,
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];
    ChooseMapLayout {
        shell,
        title: skirmish_right_anchor(dlu_rect(318, 1, 108, 10)),
        map_preview: skirmish_right_anchor(dlu_rect(324, 23, 96, 69)),
        label_engagement: dlu_rect(23, 20, 257, 12),
        label_game_type: dlu_rect(20, 60, 130, 10),
        label_game_map: dlu_rect(168, 60, 130, 10),
        game_type_list: dlu_rect(20, 78, 130, 160),
        map_list: dlu_rect(168, 78, 130, 160),
        status_help: dlu_rect(10, 282, 303, 12),
    }
}
