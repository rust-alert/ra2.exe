//! 退出确认闭环：`solve_exit_confirm` → snapshot → hit → `RenderPlan`。

use ra_layout::{solve_exit_confirm, EXIT_CONFIRM_BUTTON_IDS, Point2};
use ra_widgets::RenderPlan;

#[test]
fn exit_confirm_render_plan_rects_match_snapshot_hits() {
    let snap = solve_exit_confirm();
    let plan = RenderPlan::exit_confirm_placeholders();
    let ok_id = EXIT_CONFIRM_BUTTON_IDS[0];
    let cancel_id = EXIT_CONFIRM_BUTTON_IDS[1];

    assert_eq!(
        snap.get(ok_id).map(|e| e.layout.rect),
        plan.rect_of(ok_id)
    );
    assert_eq!(
        snap.get("dialog").map(|e| e.layout.rect),
        plan.rect_of("dialog")
    );

    let cancel = plan.rect_of(cancel_id).expect("cancel");
    let hit = snap
        .hit_test(Point2 {
            x: cancel.x + 4.0,
            y: cancel.y + 4.0,
        })
        .expect("hit cancel");
    assert_eq!(hit.id.0, cancel_id);
    assert_eq!(plan.rect_of(cancel_id), Some(hit.layout.rect));
}
