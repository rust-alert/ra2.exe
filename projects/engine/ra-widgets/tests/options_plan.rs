//! 选项整页闭环：`options_page_layout_tree` → snapshot → hit → `RenderPlan`。

use ra_layout::{
    options_page_layout_tree, shell_design_size, LayoutEngine, Point2, RightPanelChrome, Viewport,
};
use ra_widgets::{options_dialog::OptionsDialogLayout, RenderPlan};

#[test]
fn options_page_render_plan_rects_match_snapshot_hits() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: shell_design_size(chrome),
            ..Viewport::default()
        },
        &options_page_layout_tree(chrome),
    );
    let plan = RenderPlan::options_page_placeholders();
    let legacy = OptionsDialogLayout::new();

    assert_eq!(
        plan.rect_of("accept")
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((
            legacy.rail[0].x,
            legacy.rail[0].y,
            legacy.rail[0].w,
            legacy.rail[0].h
        ))
    );
    assert_eq!(
        plan.rect_of("track_detail")
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((
            legacy.track_detail.x,
            legacy.track_detail.y,
            legacy.track_detail.w,
            legacy.track_detail.h
        ))
    );
    assert_eq!(
        snap.get("content").map(|e| e.layout.rect),
        plan.rect_of("content")
    );

    let hit = snap
        .hit_test(Point2 {
            x: legacy.rail[0].x as f32 + 4.0,
            y: legacy.rail[0].y as f32 + 4.0,
        })
        .expect("hit accept");
    assert_eq!(hit.id.0, "accept");
    assert_eq!(plan.rect_of("accept"), Some(hit.layout.rect));
}
