//! 装载页闭环：`load_screen_layout_tree` → snapshot → hit → `RenderPlan`。

use ra_layout::{
    load_screen_layout, load_screen_layout_tree, shell_design_size, LayoutEngine,
    LOAD_SCREEN_BUTTON_IDS, Point2, RightPanelChrome, Viewport,
};
use ra_widgets::RenderPlan;

#[test]
fn load_screen_render_plan_rects_match_snapshot_hits() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: shell_design_size(chrome),
            ..Viewport::default()
        },
        &load_screen_layout_tree(chrome),
    );
    let plan = RenderPlan::load_screen_placeholders();
    let legacy = load_screen_layout(800, 600);
    let retry = LOAD_SCREEN_BUTTON_IDS[0];
    let cancel = LOAD_SCREEN_BUTTON_IDS[1];

    assert_eq!(
        plan.rect_of(retry)
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((
            legacy.buttons[0].x,
            legacy.buttons[0].y,
            legacy.buttons[0].w,
            legacy.buttons[0].h
        ))
    );
    assert_eq!(
        snap.get("brief").map(|e| e.layout.rect),
        plan.rect_of("brief")
    );

    let hit = snap
        .hit_test(Point2 {
            x: legacy.buttons[1].x as f32 + 4.0,
            y: legacy.buttons[1].y as f32 + 4.0,
        })
        .expect("hit cancel");
    assert_eq!(hit.id.0, cancel);
    assert_eq!(plan.rect_of(cancel), Some(hit.layout.rect));
}
