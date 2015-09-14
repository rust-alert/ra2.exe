//! 选项整页闭环：`solve_options_page` → snapshot → hit → `RenderPlan`。

use ra_layout::{solve_options_page, Point2};
use ra_widgets::{options_dialog::OptionsDialogLayout, RenderPlan};

#[test]
fn options_page_render_plan_rects_match_snapshot_hits() {
    let snap = solve_options_page();
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
