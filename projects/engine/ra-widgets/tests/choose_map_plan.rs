//! 选图闭环：`solve_choose_map` → snapshot → hit → `RenderPlan`。

use ra_layout::{solve_choose_map, Point2, Rect};
use ra_widgets::RenderPlan;

#[test]
fn choose_map_render_plan_rects_match_snapshot_hits() {
    let snap = solve_choose_map();
    let plan = RenderPlan::choose_map_placeholders();

    assert_eq!(
        plan.rect_of("use_map"),
        Some(Rect::from_xywh(644.0, 241.0, 156.0, 42.0))
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
            y: 260.0,
        })
        .expect("hit use_map");
    assert_eq!(hit.id.0, "use_map");
    assert_eq!(plan.rect_of("use_map"), Some(hit.layout.rect));
}
