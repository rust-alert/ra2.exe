//! 遭遇战大厅布局。

use super::*;
use crate::{
    dialog_layout_tree, LayoutEngine, LayoutSnapshot, Rect, RightPanelChrome, Viewport,
};
use ra_types::dialog_template_0x102;

pub(super) fn dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx {
    let r = crate::reference::DluRect::new(x, y, w, h)
        .to_design_px(crate::reference::MS_SANS_SERIF_8PT);
    RectPx::new(r.x as i32, r.y as i32, r.width as i32, r.height as i32)
}

/// 遭遇战 / 选图右栏地图名底板（`sdmpbtn`）。
///
/// 几何对齐壳层布局：贴右缘，底边落在第一根 `sdbtnbkgd` 格下沿。
pub fn sdmpbtn_rect() -> RectPx {
    RectPx::new(
        SHELL_BASE_W - SDMPBTN_W,
        RIGHT_PANEL_TOP_H + RIGHT_PANEL_TILE_H - SDMPBTN_H,
        SDMPBTN_W,
        SDMPBTN_H,
    )
}

pub(super) fn combo_face(dlu: RectPx) -> RectPx {
    RectPx::new(dlu.x, dlu.y, dlu.w, SKIRMISH_COMBO_FACE_H)
}

fn rect_px(snap: &LayoutSnapshot, id: &str) -> RectPx {
    let Rect {
        x,
        y,
        width,
        height,
    } = snap.get(id).map(|e| e.layout.rect).unwrap_or_default();
    RectPx::new(x as i32, y as i32, width as i32, height as i32)
}

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

/// 遭遇战大厅布局（800×600；标量几何来自 `0x102` snapshot，行网格仍用 DLU）。
pub fn skirmish_lobby_layout(viewport_w: u32, viewport_h: u32) -> SkirmishLobbyLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    shell.lower_strip = RectPx::new(0, 0, 0, 0);
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: crate::shell_design_size(chrome),
            ..Viewport::default()
        },
        &dialog_layout_tree("dialog_0x102", &dialog_template_0x102(), chrome),
    );
    shell.buttons = [
        rect_px(&snap, "start"),
        rect_px(&snap, "choose_map"),
        rect_px(&snap, "back"),
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
        map_preview: rect_px(&snap, "map_preview"),
        title: rect_px(&snap, "title"),
        map_name_plate: rect_px(&snap, "map_name_plate"),
        game_type: rect_px(&snap, "game_type"),
        map_label: rect_px(&snap, "map_label"),
        player_name: rect_px(&snap, "player_name"),
        flags,
        side_faces,
        color_faces,
        ai_faces,
        checkboxes: [
            rect_px(&snap, "checkbox_quick"),
            rect_px(&snap, "checkbox_1"),
            rect_px(&snap, "checkbox_2"),
            rect_px(&snap, "checkbox_3"),
            rect_px(&snap, "checkbox_4"),
        ],
        track_speed: rect_px(&snap, "track_speed"),
        track_credits: rect_px(&snap, "track_credits"),
        track_units: rect_px(&snap, "track_units"),
        label_speed: rect_px(&snap, "label_speed"),
        label_credits: rect_px(&snap, "label_credits"),
        label_units: rect_px(&snap, "label_units"),
        status_help: rect_px(&snap, "status_help"),
    }
}
