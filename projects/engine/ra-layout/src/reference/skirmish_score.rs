//! 遭遇战积分页：dialog `0x108` DLU → 壳层右栏 + 左区积分表。
//!
//! 几何权威为零售 `RT_DIALOG`（MS Sans Serif 8pt，base 6×13）。右栏 chrome 与主菜单同族；
//! **不是**对局 HUD，也**不是**暂停菜单 layout。表直接画在壳层背景上（无 `mp*scrnl` 几何）。

use crate::{
    geometry::Rect,
    policy::{RightPanelChrome, right_panel_anchor, tile_snap_button},
    reference::{
        DluRect, MS_SANS_SERIF_8PT,
        from_template::shell_design_size,
        shell_chrome::{TOOLTIP_BOTTOM_GAP, TOOLTIP_H, TOOLTIP_W, TOOLTIP_X, shell_panel_chrome_children, solve_with_shell_defaults},
    },
    shell::SKIRMISH_SCORE_BUTTON_IDS,
    snapshot::LayoutSnapshot,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
};

/// 模板声明的玩家行槽数。
pub const SCORE_ROW_SLOTS: usize = 8;

/// 行距（DLU）。
const ROW_PITCH_DLU: i32 = 22;
/// 首行玩家顶边（DLU）。
const FIRST_ROW_TOP_DLU: i32 = 120;
/// 表头顶边（DLU）。
const HEADER_TOP_DLU: i32 = 98;
/// Game / Time 摘要行顶边（DLU）。
const SUMMARY_TOP_DLU: i32 = 76;
/// 单元格高（DLU）。
const CELL_H_DLU: i32 = 10;

const COL_NAME_DLU: (i32, i32) = (66, 75);
const COL_KILLS_DLU: (i32, i32) = (147, 45);
const COL_LOSSES_DLU: (i32, i32) = (202, 45);
const COL_BUILT_DLU: (i32, i32) = (257, 45);
const COL_SCORE_DLU: (i32, i32) = (308, 45);
const GAME_LABEL_DLU: (i32, i32) = (66, 115);
const TIME_LABEL_DLU: (i32, i32) = (248, 105);
/// 右栏标题（比其它壳层标题左偏 1 DLU）。
const TITLE_DLU: DluRect = DluRect::new(424, 1, 108, 10);
/// 「继续」owner-draw 资源 DLU。
const CONTINUE_DLU: DluRect = DluRect::new(425, 326, 108, 23);

fn cell_dlu(col: (i32, i32), top_dlu: i32) -> Rect {
    DluRect::new(col.0, top_dlu, col.1, CELL_H_DLU).to_design_px(MS_SANS_SERIF_8PT)
}

fn push_row_cells(children: &mut Vec<LayoutNode>, prefix: &str, top_dlu: i32) {
    children.push(fixed_rect_leaf(format!("{prefix}_name"), cell_dlu(COL_NAME_DLU, top_dlu)));
    children.push(fixed_rect_leaf(format!("{prefix}_kills"), cell_dlu(COL_KILLS_DLU, top_dlu)));
    children.push(fixed_rect_leaf(format!("{prefix}_losses"), cell_dlu(COL_LOSSES_DLU, top_dlu)));
    children.push(fixed_rect_leaf(format!("{prefix}_built"), cell_dlu(COL_BUILT_DLU, top_dlu)));
    children.push(fixed_rect_leaf(format!("{prefix}_score"), cell_dlu(COL_SCORE_DLU, top_dlu)));
}

/// 遭遇战积分整页：壳层面板 chrome + `0x108` 表几何 + 右栏「继续」。
pub(crate) fn skirmish_score_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_panel_chrome_children(chrome);
    // 标题走 `0x108` DLU + 右栏锚，**不要**复用主菜单壳层默认 title 框。
    children.push(fixed_rect_leaf("title", right_panel_anchor(TITLE_DLU.to_design_px(MS_SANS_SERIF_8PT), chrome)));
    children.push(fixed_rect_leaf(
        "tooltip",
        Rect::from_xywh(TOOLTIP_X, chrome.shell_h - TOOLTIP_H - TOOLTIP_BOTTOM_GAP, TOOLTIP_W, TOOLTIP_H),
    ));
    children.push(fixed_rect_leaf("game_label", cell_dlu(GAME_LABEL_DLU, SUMMARY_TOP_DLU)));
    children.push(fixed_rect_leaf("time_label", cell_dlu(TIME_LABEL_DLU, SUMMARY_TOP_DLU)));
    push_row_cells(&mut children, "header", HEADER_TOP_DLU);
    for slot in 0..SCORE_ROW_SLOTS {
        push_row_cells(&mut children, &format!("row{slot}"), FIRST_ROW_TOP_DLU + ROW_PITCH_DLU * slot as i32);
    }
    // 资源 DLU 经 tile 吸附到右栏 SDBTNANM 格（800×600 与贴底盖同格）。
    let continue_btn = tile_snap_button(CONTINUE_DLU.to_design_px(MS_SANS_SERIF_8PT), chrome);
    children.push(fixed_rect_leaf(SKIRMISH_SCORE_BUTTON_IDS[0], continue_btn));
    root_with_fixed_children("skirmish_score", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解遭遇战积分页。
pub fn solve_skirmish_score() -> LayoutSnapshot {
    solve_with_shell_defaults(skirmish_score_content_layout_tree)
}
