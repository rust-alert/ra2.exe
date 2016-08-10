//! 遭遇战积分页：壳层右栏 + 左区统计表。

use crate::{
    geometry::Rect,
    policy::{RightPanelChrome, bottom_cover_button},
    reference::{
        from_template::shell_design_size,
        shell_chrome::{shell_chrome_children, solve_with_shell_defaults},
    },
    shell::SKIRMISH_SCORE_BUTTON_IDS,
    snapshot::LayoutSnapshot,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
};

/// 遭遇战积分整页：壳层 chrome + 统计区 + 右栏「继续」。
pub(crate) fn skirmish_score_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_chrome_children(chrome);
    children.extend([
        // 左区统计卡：底板与全部文案共用此框（含游戏/时间/表头）。
        fixed_rect_leaf("stats_panel", Rect::from_xywh(36.0, 56.0, 560.0, 300.0)),
        fixed_rect_leaf("game_label", Rect::from_xywh(52.0, 68.0, 180.0, 20.0)),
        fixed_rect_leaf("time_label", Rect::from_xywh(360.0, 68.0, 220.0, 20.0)),
        // 表区相对面板内缩，避免「玩家」贴边出框。
        fixed_rect_leaf("table", Rect::from_xywh(52.0, 96.0, 528.0, 244.0)),
        fixed_rect_leaf(SKIRMISH_SCORE_BUTTON_IDS[0], bottom_cover_button(chrome)),
    ]);
    root_with_fixed_children("skirmish_score", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解遭遇战积分页。
pub fn solve_skirmish_score() -> LayoutSnapshot {
    solve_with_shell_defaults(skirmish_score_content_layout_tree)
}
