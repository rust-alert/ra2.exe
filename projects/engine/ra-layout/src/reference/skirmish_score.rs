//! 遭遇战积分页：壳层右栏 + 左区统计表。

use crate::{
    geometry::Rect,
    policy::{bottom_cover_button, RightPanelChrome},
    reference::from_template::shell_design_size,
    reference::shell_chrome::{shell_chrome_children, solve_with_shell_defaults},
    snapshot::LayoutSnapshot,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
    shell::SKIRMISH_SCORE_BUTTON_IDS,
};

/// 遭遇战积分整页：壳层 chrome + 统计区 + 右栏「继续」。
pub(crate) fn skirmish_score_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_chrome_children(chrome);
    children.extend([
        // 左区统计卡（相对 800×600 设计坐标，避开右栏）。
        fixed_rect_leaf("stats_panel", Rect::from_xywh(40.0, 64.0, 560.0, 340.0)),
        fixed_rect_leaf("game_label", Rect::from_xywh(52.0, 76.0, 160.0, 20.0)),
        // 与表右缘对齐，便于「時間」右齐。
        fixed_rect_leaf("time_label", Rect::from_xywh(360.0, 76.0, 224.0, 20.0)),
        fixed_rect_leaf("table", Rect::from_xywh(52.0, 108.0, 532.0, 270.0)),
        fixed_rect_leaf(SKIRMISH_SCORE_BUTTON_IDS[0], bottom_cover_button(chrome)),
    ]);
    root_with_fixed_children("skirmish_score", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解遭遇战积分页。
pub fn solve_skirmish_score() -> LayoutSnapshot {
    solve_with_shell_defaults(skirmish_score_content_layout_tree)
}
