//! 战役页闭环：`solve_campaign` → snapshot → hit → `RenderPlan`。

use ra_layout::{campaign_layout, solve_campaign, CAMPAIGN_SIDE_IDS, Point2};
use ra_widgets::RenderPlan;

#[test]
fn campaign_render_plan_rects_match_snapshot_hits() {
    let snap = solve_campaign();
    let plan = RenderPlan::campaign_placeholders();
    let legacy = campaign_layout(800, 600);
    let allied = CAMPAIGN_SIDE_IDS[0];

    assert_eq!(
        plan.rect_of(allied)
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((legacy.allied.x, legacy.allied.y, legacy.allied.w, legacy.allied.h))
    );
    assert_eq!(
        snap.get("difficulty").map(|e| e.layout.rect),
        plan.rect_of("difficulty")
    );

    let hit = snap
        .hit_test(Point2 {
            x: legacy.allied.x as f32 + 4.0,
            y: legacy.allied.y as f32 + 4.0,
        })
        .expect("hit allied");
    assert_eq!(hit.id.0, allied);
    assert_eq!(plan.rect_of(allied), Some(hit.layout.rect));
}
