//! 遭遇战大厅闭环：`solve_skirmish_lobby` → snapshot → hit → `RenderPlan`。

use ra_layout::{solve_skirmish_lobby, skirmish_lobby_layout, Point2};
use ra_widgets::RenderPlan;

#[test]
fn skirmish_lobby_render_plan_rects_match_snapshot_hits() {
    let snap = solve_skirmish_lobby();
    let plan = RenderPlan::shell_dialog_placeholders(0x102, "dialog_0x102");
    let legacy = skirmish_lobby_layout(800, 600);

    assert_eq!(
        plan.rect_of("start")
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((
            legacy.shell.buttons[0].x,
            legacy.shell.buttons[0].y,
            legacy.shell.buttons[0].w,
            legacy.shell.buttons[0].h
        ))
    );
    assert_eq!(
        snap.get("map_preview").map(|e| e.layout.rect),
        plan.rect_of("map_preview")
    );

    let hit = snap
        .hit_test(Point2 {
            x: legacy.shell.buttons[0].x as f32 + 4.0,
            y: legacy.shell.buttons[0].y as f32 + 4.0,
        })
        .expect("hit start");
    assert_eq!(hit.id.0, "start");
    assert_eq!(plan.rect_of("start"), Some(hit.layout.rect));
}
