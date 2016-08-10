//! 战役页内容区固定设计矩形 → `LayoutNode`。

use crate::{
    geometry::Rect,
    policy::{RightPanelChrome, bottom_cover_button},
    reference::{
        from_template::shell_design_size,
        shell_chrome::{shell_chrome_children, solve_with_shell_defaults},
    },
    shell::{
        CAMPAIGN_ALLIED_ORIGIN, CAMPAIGN_ALLIED_SIZE, CAMPAIGN_BUTTON_IDS, CAMPAIGN_SIDE_IDS, CAMPAIGN_SOVIET_ORIGIN, CAMPAIGN_SOVIET_SIZE,
        CAMPAIGN_TUTORIAL_ORIGIN, CAMPAIGN_TUTORIAL_SIZE,
    },
    snapshot::LayoutSnapshot,
    spec::{LayoutNode, fixed_rect_leaf, root_with_fixed_children},
};

pub fn side_rect(origin: (i32, i32), size: (i32, i32)) -> Rect {
    Rect::from_xywh(origin.0 as f32, origin.1 as f32, size.0 as f32, size.1 as f32)
}

/// 战役整页：壳层 chrome + 三侧入口 + 难度区 + 右栏「上一页」，一次求解。
pub fn campaign_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_chrome_children(chrome);
    children.extend([
        fixed_rect_leaf(CAMPAIGN_SIDE_IDS[0], side_rect(CAMPAIGN_ALLIED_ORIGIN, CAMPAIGN_ALLIED_SIZE)),
        fixed_rect_leaf(CAMPAIGN_SIDE_IDS[1], side_rect(CAMPAIGN_TUTORIAL_ORIGIN, CAMPAIGN_TUTORIAL_SIZE)),
        fixed_rect_leaf(CAMPAIGN_SIDE_IDS[2], side_rect(CAMPAIGN_SOVIET_ORIGIN, CAMPAIGN_SOVIET_SIZE)),
        // 难度区：侧图下方固定槽位。
        fixed_rect_leaf("difficulty_label", Rect::from_xywh(191.0, 454.0, 100.0, 20.0)),
        fixed_rect_leaf("difficulty_value", Rect::from_xywh(338.0, 454.0, 100.0, 20.0)),
        fixed_rect_leaf("difficulty", Rect::from_xywh(191.0, 483.0, 247.0, 13.0)),
        fixed_rect_leaf(CAMPAIGN_BUTTON_IDS[0], bottom_cover_button(chrome)),
    ]);
    root_with_fixed_children("campaign", shell_design_size(chrome), children)
}

/// 用壳层默认 chrome 求解 [`campaign_content_layout_tree`]。
pub fn solve_campaign() -> LayoutSnapshot {
    solve_with_shell_defaults(campaign_content_layout_tree)
}
