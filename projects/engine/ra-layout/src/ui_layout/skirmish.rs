//! 遭遇战大厅布局。

use super::*;
use crate::solve_skirmish_lobby;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkirmishLobbyLayout {
    /// 共用壳层 chrome（背景 / 右栏）。遭遇战不画底条装饰。
    pub shell: MainMenuLayout,
    /// 右栏小地图预览（`0x468`）。
    pub map_preview: RectPx,
    /// 右栏标题（`0x694` / `GUI:SkirmishGame`）。
    pub title: RectPx,
    /// 右栏地图名底板（`sdmpbtn`）。
    pub map_name_plate: RectPx,
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

/// 遭遇战大厅布局（800×600；几何权威来自 `0x102` snapshot）。
pub fn skirmish_lobby_layout(_viewport_w: u32, _viewport_h: u32) -> SkirmishLobbyLayout {
    let mut shell = shell_chrome_base_layout();
    shell.lower_strip = RectPx::new(0, 0, 0, 0);
    let snap = solve_skirmish_lobby();
    shell.buttons = [
        rect_px_from_snapshot(&snap, "start"),
        rect_px_from_snapshot(&snap, "choose_map"),
        rect_px_from_snapshot(&snap, "back"),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
        RectPx::new(0, 0, 0, 0),
    ];

    let mut flags = [RectPx::new(0, 0, 0, 0); SKIRMISH_ROW_COUNT];
    let mut side_faces = [RectPx::new(0, 0, 0, 0); SKIRMISH_ROW_COUNT];
    let mut color_faces = [RectPx::new(0, 0, 0, 0); SKIRMISH_ROW_COUNT];
    let mut ai_faces = [RectPx::new(0, 0, 0, 0); SKIRMISH_AI_ROW_COUNT];
    for i in 0..SKIRMISH_ROW_COUNT {
        flags[i] = rect_px_from_snapshot(&snap, &format!("flag_{i}"));
        side_faces[i] = rect_px_from_snapshot(&snap, &format!("side_face_{i}"));
        color_faces[i] = rect_px_from_snapshot(&snap, &format!("color_face_{i}"));
    }
    for i in 0..SKIRMISH_AI_ROW_COUNT {
        ai_faces[i] = rect_px_from_snapshot(&snap, &format!("ai_face_{i}"));
    }

    SkirmishLobbyLayout {
        shell,
        map_preview: rect_px_from_snapshot(&snap, "map_preview"),
        title: rect_px_from_snapshot(&snap, "title"),
        map_name_plate: rect_px_from_snapshot(&snap, "map_name_plate"),
        game_type: rect_px_from_snapshot(&snap, "game_type"),
        map_label: rect_px_from_snapshot(&snap, "map_label"),
        player_name: rect_px_from_snapshot(&snap, "player_name"),
        flags,
        side_faces,
        color_faces,
        ai_faces,
        checkboxes: [
            rect_px_from_snapshot(&snap, "checkbox_quick"),
            rect_px_from_snapshot(&snap, "checkbox_1"),
            rect_px_from_snapshot(&snap, "checkbox_2"),
            rect_px_from_snapshot(&snap, "checkbox_3"),
            rect_px_from_snapshot(&snap, "checkbox_4"),
        ],
        track_speed: rect_px_from_snapshot(&snap, "track_speed"),
        track_credits: rect_px_from_snapshot(&snap, "track_credits"),
        track_units: rect_px_from_snapshot(&snap, "track_units"),
        label_speed: rect_px_from_snapshot(&snap, "label_speed"),
        label_credits: rect_px_from_snapshot(&snap, "label_credits"),
        label_units: rect_px_from_snapshot(&snap, "label_units"),
        status_help: rect_px_from_snapshot(&snap, "status_help"),
    }
}
