//! 遭遇战大厅布局。

use super::*;


pub(super) fn dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx {
    // MS Sans Serif 8pt：x×6/4、y×13/8，四舍五入。
    fn mul_div_round(n: i32, numer: i32, denom: i32) -> i32 {
        let value = n * numer;
        if value >= 0 { (value + denom / 2) / denom } else { (value - denom / 2) / denom }
    }
    RectPx::new(mul_div_round(x, 6, 4), mul_div_round(y, 13, 8), mul_div_round(w, 6, 4), mul_div_round(h, 13, 8))
}

pub(super) fn skirmish_snap_button(source: RectPx, panel_tile_y: i32) -> RectPx {
    // 偏置截断：相对 `sdbtnbkgd` 列顶，按 42px 格吸附（壳层 chrome，非截图估）。
    let tile_h = RIGHT_PANEL_TILE_H.max(1);
    let tile_index = ((source.y - panel_tile_y + tile_h / 2) / tile_h).max(0);
    button_cell(SHELL_BASE_W - RIGHT_PANEL_W, panel_tile_y + tile_index * tile_h)
}

pub(super) fn skirmish_right_anchor(base: RectPx) -> RectPx {
    // 右栏静态/预览：相对 `RIGHT_PANEL_W` 水平居中锚到右缘。
    let inset = (RIGHT_PANEL_W - base.w) / 2;
    RectPx::new(SHELL_BASE_W - base.w - inset, base.y, base.w, base.h)
}

/// 遭遇战 / 选图右栏地图名底板（`sdmpbtn`）。
///
/// 几何对齐壳层布局：贴右缘，底边落在第一根 `sdbtnbkgd` 格下沿。
pub fn sdmpbtn_rect() -> RectPx {
    RectPx::new(SHELL_BASE_W - SDMPBTN_W, RIGHT_PANEL_TOP_H + RIGHT_PANEL_TILE_H - SDMPBTN_H, SDMPBTN_W, SDMPBTN_H)
}

pub(super) fn combo_face(dlu: RectPx) -> RectPx {
    RectPx::new(dlu.x, dlu.y, dlu.w, SKIRMISH_COMBO_FACE_H)
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

/// 遭遇战大厅布局（800×600 内容坐标；DLU→px 用 MS Sans Serif 8pt）。
pub fn skirmish_lobby_layout(viewport_w: u32, viewport_h: u32) -> SkirmishLobbyLayout {
    let mut shell = main_menu_layout(viewport_w, viewport_h);
    // 遭遇战无 `lwscrnl` 底条；右栏三钮：开始 / 选图 / 返回（贴底盖格）。
    shell.lower_strip = RectPx::new(0, 0, 0, 0);
    let start = skirmish_snap_button(dlu_rect(318, 149, 108, 23), shell.panel_tile.y);
    let choose = skirmish_snap_button(dlu_rect(318, 176, 108, 23), shell.panel_tile.y);
    // 返回：壳层贴底盖上沿一行（owner-draw 底行惯例），不用对话框里偏上的 `0x5C0` y。
    let back = button_cell(shell.panel_top.x, shell.panel_bottom.y - BUTTON_CELL_H);
    shell.buttons = [start, choose, back, RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0), RectPx::new(0, 0, 0, 0)];

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
        map_name_plate: sdmpbtn_rect(),
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
