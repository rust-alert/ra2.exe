//! 战役页内容区固定设计矩形 → `LayoutNode`。

use crate::{
    geometry::Rect,
    policy::{bottom_cover_button, RightPanelChrome},
    reference::from_template::shell_design_size,
    reference::shell_chrome::{shell_chrome_children, solve_with_shell_defaults},
    snapshot::LayoutSnapshot,
    spec::{fixed_rect_leaf, root_with_fixed_children, LayoutNode},
    shell::{
        CAMPAIGN_ALLIED_ORIGIN, CAMPAIGN_ALLIED_SIZE, CAMPAIGN_BUTTON_IDS, CAMPAIGN_SIDE_IDS,
        CAMPAIGN_SOVIET_ORIGIN, CAMPAIGN_SOVIET_SIZE, CAMPAIGN_TUTORIAL_ORIGIN, CAMPAIGN_TUTORIAL_SIZE,
    },
};

fn side_rect(origin: (i32, i32), size: (i32, i32)) -> Rect {
    Rect::from_xywh(origin.0 as f32, origin.1 as f32, size.0 as f32, size.1 as f32)
}

/// 战役整页：壳层 chrome + 三侧入口 + 难度区 + 右栏「上一页」，一次求解。
pub(crate) fn campaign_content_layout_tree(chrome: RightPanelChrome) -> LayoutNode {
    let mut children = shell_chrome_children(chrome);
    children.extend([
        fixed_rect_leaf(
            CAMPAIGN_SIDE_IDS[0],
            side_rect(CAMPAIGN_ALLIED_ORIGIN, CAMPAIGN_ALLIED_SIZE),
        ),
        fixed_rect_leaf(
            CAMPAIGN_SIDE_IDS[1],
            side_rect(CAMPAIGN_TUTORIAL_ORIGIN, CAMPAIGN_TUTORIAL_SIZE),
        ),
        fixed_rect_leaf(
            CAMPAIGN_SIDE_IDS[2],
            side_rect(CAMPAIGN_SOVIET_ORIGIN, CAMPAIGN_SOVIET_SIZE),
        ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayoutEngine, Viewport};

    #[test]
    fn campaign_content_matches_golden_shell_slots() {
        use crate::shell::{rect_px_from_snapshot, RectPx};

        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &campaign_content_layout_tree(chrome),
        );
        // 侧图原点相对 `fsbkgdlg`；难度轨在苏军图下方；右栏「上一页」贴底盖。
        assert_eq!(
            rect_px_from_snapshot(&snap, "allied"),
            RectPx::new(30, 26, 570, 135)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "tutorial"),
            RectPx::new(82, 187, 468, 108)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "soviet"),
            RectPx::new(98, 298, 444, 149)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "difficulty_label"),
            RectPx::new(191, 454, 100, 20)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "difficulty_value"),
            RectPx::new(338, 454, 100, 20)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "difficulty"),
            RectPx::new(191, 483, 247, 13)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "back"),
            RectPx::new(644, 535, 156, 42)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "title"),
            RectPx::new(635, 9, 163, 18)
        );
        assert_eq!(
            rect_px_from_snapshot(&snap, "tooltip"),
            RectPx::new(10, 579, 455, 20)
        );
    }
}
