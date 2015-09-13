//! 网络占位页闭环：`solve_network_page` → snapshot → hit → `RenderPlan`。

use ra_layout::{solve_network_page, Point2, NETWORK_BUTTON_IDS};
use ra_widgets::RenderPlan;

#[test]
fn network_render_plan_rects_match_snapshot_hits() {
    let snap = solve_network_page();
    let plan = RenderPlan::network_page_placeholders();

    for id in NETWORK_BUTTON_IDS {
        assert_eq!(
            snap.get(id).map(|e| e.layout.rect),
            plan.rect_of(id),
            "{id}"
        );
    }

    let hit = snap
        .hit_test(Point2 {
            x: 200.0,
            y: 350.0,
        })
        .expect("hit back");
    assert_eq!(hit.id.0, "back");
    assert_eq!(plan.rect_of("back"), Some(hit.layout.rect));
}
