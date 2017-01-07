//! 选项整页闭环：`solve_options_page` → snapshot → hit → `RenderPlan`。

use ra_layout::{Point2, solve_options_page};
use ra_widgets::RenderPlan;

#[test]
fn options_page_render_plan_rects_match_snapshot_hits() {
    let snap = solve_options_page();
    let plan = RenderPlan::options_page_placeholders();

    assert_eq!(snap.get("keyboard").map(|e| e.layout.rect), plan.rect_of("keyboard"));
    assert_eq!(snap.get("track_detail").map(|e| e.layout.rect), plan.rect_of("track_detail"));
    assert_eq!(snap.get("caption_detail").map(|e| e.layout.rect), plan.rect_of("caption_detail"));

    let keyboard = plan.rect_of("keyboard").expect("keyboard");
    let hit = snap.hit_test(Point2 { x: keyboard.x + 4.0, y: keyboard.y + 4.0 }).expect("hit keyboard");
    assert_eq!(hit.id.0, "keyboard");
    assert_eq!(plan.rect_of("keyboard"), Some(hit.layout.rect));
}
