//! 选项整页闭环：`solve_options_page` → snapshot → hit → `RenderPlan`。

use ra_layout::{solve_options_page, Point2};
use ra_widgets::RenderPlan;

#[test]
fn options_page_render_plan_rects_match_snapshot_hits() {
    let snap = solve_options_page();
    let plan = RenderPlan::options_page_placeholders();

    assert_eq!(
        snap.get("accept").map(|e| e.layout.rect),
        plan.rect_of("accept")
    );
    assert_eq!(
        snap.get("track_detail").map(|e| e.layout.rect),
        plan.rect_of("track_detail")
    );
    assert_eq!(
        snap.get("content").map(|e| e.layout.rect),
        plan.rect_of("content")
    );

    let accept = plan.rect_of("accept").expect("accept");
    let hit = snap
        .hit_test(Point2 {
            x: accept.x + 4.0,
            y: accept.y + 4.0,
        })
        .expect("hit accept");
    assert_eq!(hit.id.0, "accept");
    assert_eq!(plan.rect_of("accept"), Some(hit.layout.rect));
}
