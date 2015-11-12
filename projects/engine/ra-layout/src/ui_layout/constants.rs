//! 壳层布局常量。


/// 壳层设计宽。
pub const SHELL_BASE_W: i32 = 800;

/// 壳层设计高。
pub const SHELL_BASE_H: i32 = 600;

/// 右侧面板宽。
pub const RIGHT_PANEL_W: i32 = 168;

/// 右侧顶盖高（`sdtp`）。
pub const RIGHT_PANEL_TOP_H: i32 = 199;

/// WARNING 窗内动画（`sdwrnanm`）画布宽。
pub const SDWRNANM_W: i32 = 92;

/// WARNING 窗内动画（`sdwrnanm`）画布高。
pub const SDWRNANM_H: i32 = 53;

/// `sdwrnanm` 相对 `sdtp` 左上角的 X 偏移（窗内可视区）。
pub const SDWRNANM_OFFSET_X: i32 = 38;

/// `sdwrnanm` 相对 `sdtp` 左上角的 Y 偏移（窗内可视区）。
pub const SDWRNANM_OFFSET_Y: i32 = 48;

/// 遭遇战地图名底板宽（`sdmpbtn`）。
pub const SDMPBTN_W: i32 = 156;

/// 遭遇战地图名底板高（`sdmpbtn`）。
pub const SDMPBTN_H: i32 = 84;

/// 右侧平铺条高（`sdbtnbkgd`）。
pub const RIGHT_PANEL_TILE_H: i32 = 42;

/// 按钮格宽（窄列）。
pub const BUTTON_CELL_W: i32 = 156;

/// 按钮格高。
pub const BUTTON_CELL_H: i32 = 42;

/// 底部装饰条高（`lwscrnl`）。
pub const LOWER_STRIP_H: i32 = 32;

/// 主菜单按钮入口 id（与 `ra_widgets::skin::slots` 顺序一致）。
///
/// 顺序对齐原版 0xE2：前五项连续平铺格，末项 Exit 贴底盖。
pub const MAIN_MENU_BUTTON_IDS: [&str; 6] = ["single_player", "ww_online", "network", "movies", "options", "exit"];

/// 单人页按钮入口 id（与 `ra_widgets::skin::slots` 顺序一致）。
///
/// 顺序：新战役 → 载入 → 遭遇战 → 主菜单（贴底盖）。
pub const SINGLE_PLAYER_BUTTON_IDS: [&str; 4] = ["campaign", "load", "skirmish", "back"];

/// 战役页右栏按钮入口 id（与 `ra_widgets::skin::slots` 顺序一致）。
///
/// 原版 `0x94` 右栏为部队格 + 唯一「上一页」；无「载入」钮。
pub const CAMPAIGN_BUTTON_IDS: [&str; 1] = ["back"];

/// 战役三侧入口 id（盟军 / 新兵训练营 / 苏军；`battle.ini` 的 ALL1 / TUT1 / SOV1）。
pub const CAMPAIGN_SIDE_IDS: [&str; 3] = ["allied", "tutorial", "soviet"];

/// 遭遇战大厅右侧按钮入口 id（与 `ra_widgets::skin::slots` 顺序一致）。
///
/// 顺序对齐原版 0x102：开始游戏 → 选图（自订战役）→ 上一页（贴底）。
pub const SKIRMISH_LOBBY_BUTTON_IDS: [&str; 3] = ["start", "choose_map", "back"];

/// 选图页右侧按钮入口 id（与 `ra_widgets::skin::slots` 顺序一致）。
///
/// 顺序对齐安装 `game.exe` 的 `RT_DIALOG` id=`107`（`0x6B`）：
/// 使用地图 → 创建随机地图 → 取消（贴底）。
pub const CHOOSE_MAP_BUTTON_IDS: [&str; 3] = ["use_map", "create_random", "cancel"];

/// 选项页按钮入口 id（与 `ra_widgets::skin::slots` 顺序一致）。
pub const OPTIONS_BUTTON_IDS: [&str; 3] = ["accept", "cancel", "main_menu"];

/// 退出确认对话框按钮 id。
pub const EXIT_CONFIRM_BUTTON_IDS: [&str; 2] = ["ok", "cancel"];

/// 遭遇战玩家行数（本地 + AI）。
pub const SKIRMISH_ROW_COUNT: usize = 8;

/// 遭遇战 AI 行数（不含本地玩家）。
pub const SKIRMISH_AI_ROW_COUNT: usize = 7;

/// 下拉面高度（像素）。
pub const SKIRMISH_COMBO_FACE_H: i32 = 24;

/// 下拉右侧箭头保留宽（像素）。
pub const SKIRMISH_COMBO_ARROW_RESERVE: i32 = 20;

/// 勾选图标宽。
pub const SKIRMISH_CHECK_W: i32 = 18;

/// 勾选图标高。
pub const SKIRMISH_CHECK_H: i32 = 18;

/// 滑条右侧数值底板宽（`trofl`/`trofm`/`trofr`）。
pub const SKIRMISH_TRACK_PLAQUE_W: i32 = 50;

/// 滑条活跃轨宽公式附加扣除（与数值底板一起得到 65）。
pub const SKIRMISH_TRACK_ACTIVE_PAD: i32 = 13;

/// 滑条拇指宽（`trakgrip.pcx`）。
pub const SKIRMISH_TRACK_THUMB_W: i32 = 12;

/// 选项页分辨率下拉展开行高。
pub const OPTIONS_RESOLUTION_ROW_H: i32 = 24;

/// `fsalg.shp` 相对 `fsbkgdlg` 左上角。
pub const CAMPAIGN_ALLIED_ORIGIN: (i32, i32) = (30, 26);

/// `fsalg.shp` 画布。
pub const CAMPAIGN_ALLIED_SIZE: (i32, i32) = (570, 135);

/// `fsbclg.shp` 相对 `fsbkgdlg` 左上角（登记对齐 y=187，勿用 186：差 1px 悬停会抖）。
pub const CAMPAIGN_TUTORIAL_ORIGIN: (i32, i32) = (82, 187);

/// `fsbclg.shp` 画布。
pub const CAMPAIGN_TUTORIAL_SIZE: (i32, i32) = (468, 108);

/// `fsslg.shp` 相对 `fsbkgdlg` 左上角。
pub const CAMPAIGN_SOVIET_ORIGIN: (i32, i32) = (98, 298);

/// `fsslg.shp` 画布。
pub const CAMPAIGN_SOVIET_SIZE: (i32, i32) = (444, 149);

/// 退出确认 MessageBox 面板宽（安装内 `pudlgbgn.shp` 画布；非 DLU 四舍五入的 450）。
pub const EXIT_CONFIRM_DIALOG_W: i32 = 451;

/// 退出确认 MessageBox 面板高（安装内 `pudlgbgn.shp` 画布；非 DLU 四舍五入的 325）。
pub const EXIT_CONFIRM_DIALOG_H: i32 = 326;

/// MessageBox 按钮艺术宽（安装内 `mnbttn.shp` 画布）。
pub const EXIT_CONFIRM_BUTTON_W: i32 = 126;

/// MessageBox 按钮艺术高（安装内 `mnbttn.shp` 画布）。
pub const EXIT_CONFIRM_BUTTON_H: i32 = 25;

/// 战斗暂停菜单右侧按钮入口 id（与 `battle_pause_menu` / 合成顺序一致）。
///
/// 顺序对齐原版 Esc 菜单：选项 → 载入 → 保存 → 重新开始 → 放弃任务；「回到游戏」贴底。
pub const BATTLE_PAUSE_MENU_BUTTON_IDS: [&str; 6] = ["options", "load", "save", "restart", "abort", "resume"];
