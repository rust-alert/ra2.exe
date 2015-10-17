//! 遭遇战大厅闭环：`solve_skirmish_lobby` → snapshot → hit → `RenderPlan`。

use ra_layout::{solve_skirmish_lobby, Point2};
use ra_widgets::RenderPlan;

#[test]
fn skirmish_lobby_render_plan_rects_match_snapshot_hits() {
    let snap = solve_skirmish_lobby();
    let plan = RenderPlan::skirmish_lobby_placeholders();
    let start = snap.get("start").expect("start").layout.rect;

    assert_eq!(plan.rect_of("start"), Some(start));
    assert_eq!(
        snap.get("map_preview").map(|e| e.layout.rect),
        plan.rect_of("map_preview")
    );

    let hit = snap
        .hit_test(Point2 {
            x: start.x + 4.0,
            y: start.y + 4.0,
        })
        .expect("hit start");
    assert_eq!(hit.id.0, "start");
    assert_eq!(plan.rect_of("start"), Some(hit.layout.rect));
}
