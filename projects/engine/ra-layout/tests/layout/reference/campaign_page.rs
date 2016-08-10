//! 自 `engine/ra-layout/src/reference/campaign_page.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/campaign_page.rs :: tests
use ra_layout::{
    LayoutEngine, RightPanelChrome, Viewport,
    reference::{campaign_page::*, from_template::shell_design_size},
};

#[test]
fn campaign_content_matches_golden_shell_slots() {
    use ra_layout::shell::{RectPx, rect_px_from_snapshot};

    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &campaign_content_layout_tree(chrome));
    // 侧图原点相对 `fsbkgdlg`；难度轨在苏军图下方；右栏「上一页」贴底盖。
    assert_eq!(rect_px_from_snapshot(&snap, "allied"), RectPx::new(30, 26, 570, 135));
    assert_eq!(rect_px_from_snapshot(&snap, "tutorial"), RectPx::new(82, 187, 468, 108));
    assert_eq!(rect_px_from_snapshot(&snap, "soviet"), RectPx::new(98, 298, 444, 149));
    assert_eq!(rect_px_from_snapshot(&snap, "difficulty_label"), RectPx::new(191, 454, 100, 20));
    assert_eq!(rect_px_from_snapshot(&snap, "difficulty_value"), RectPx::new(338, 454, 100, 20));
    assert_eq!(rect_px_from_snapshot(&snap, "difficulty"), RectPx::new(191, 483, 247, 13));
    assert_eq!(rect_px_from_snapshot(&snap, "back"), RectPx::new(644, 535, 156, 42));
    assert_eq!(rect_px_from_snapshot(&snap, "title"), RectPx::new(635, 9, 163, 18));
    assert_eq!(rect_px_from_snapshot(&snap, "tooltip"), RectPx::new(10, 579, 455, 20));
}
