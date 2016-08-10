//! 装载页闭环：`solve_load_screen` → snapshot → hit → `RenderPlan`。

use ra_layout::{LOAD_SCREEN_BUTTON_IDS, Point2, solve_load_screen};
use ra_widgets::RenderPlan;

#[test]
fn load_screen_render_plan_rects_match_snapshot_hits() {
    let snap = solve_load_screen();
    let plan = RenderPlan::load_screen_placeholders();
    let retry = LOAD_SCREEN_BUTTON_IDS[0];
    let cancel = LOAD_SCREEN_BUTTON_IDS[1];

    assert_eq!(snap.get(retry).map(|e| e.layout.rect), plan.rect_of(retry));
    assert_eq!(snap.get("brief").map(|e| e.layout.rect), plan.rect_of("brief"));

    let cancel_rect = plan.rect_of(cancel).expect("cancel");
    let hit = snap.hit_test(Point2 { x: cancel_rect.x + 4.0, y: cancel_rect.y + 4.0 }).expect("hit cancel");
    assert_eq!(hit.id.0, cancel);
    assert_eq!(plan.rect_of(cancel), Some(hit.layout.rect));
}
