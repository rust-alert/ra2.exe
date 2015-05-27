//! 选图闭环：profile `0x6B` → snapshot → hit → `RenderPlan`。

use ra_layout::{
    dialog_layout_tree, shell_design_size, LayoutEngine, Point2, Rect, RightPanelChrome, Viewport,
};
use ra_adaptor::shell_runtime_ui_profile;
use ra_widgets::RenderPlan;

#[test]
fn choose_map_render_plan_rects_match_snapshot_hits() {
    let chrome = RightPanelChrome::shell_defaults();
    let profile = shell_runtime_ui_profile();
    let template = profile.dialog(0x6B).expect("dialog 0x6B");
    let snap = LayoutEngine.solve(
        Viewport {
            size: shell_design_size(chrome),
            ..Viewport::default()
        },
        &dialog_layout_tree("dialog_0x6b", template, chrome),
    );
    let plan = RenderPlan::shell_dialog_placeholders(0x6B, "dialog_0x6b");

    assert_eq!(
        plan.rect_of("use_map"),
        Some(Rect::from_xywh(644.0, 199.0, 156.0, 42.0))
    );
    assert_eq!(
        plan.rect_of("cancel"),
        Some(Rect::from_xywh(644.0, 535.0, 156.0, 42.0))
    );
    assert_eq!(
        snap.get("use_map").map(|e| e.layout.rect),
        plan.rect_of("use_map")
    );

    let hit = snap
        .hit_test(Point2 {
            x: 650.0,
            y: 210.0,
        })
        .expect("hit use_map");
    assert_eq!(hit.id.0, "use_map");
    assert_eq!(plan.rect_of("use_map"), Some(hit.layout.rect));
}
