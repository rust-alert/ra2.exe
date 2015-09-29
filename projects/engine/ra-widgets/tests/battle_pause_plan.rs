//! 对局暂停菜单闭环：`solve_battle_pause` → snapshot → hit → `RenderPlan`。

use ra_layout::{battle_pause_menu_layout, solve_battle_pause, BATTLE_PAUSE_MENU_BUTTON_IDS, Point2};
use ra_widgets::RenderPlan;

#[test]
fn battle_pause_render_plan_rects_match_snapshot_hits() {
    let snap = solve_battle_pause();
    let plan = RenderPlan::battle_pause_placeholders();
    let legacy = battle_pause_menu_layout(800, 600);
    let resume = BATTLE_PAUSE_MENU_BUTTON_IDS[5];

    assert_eq!(
        plan.rect_of(resume)
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((
            legacy.buttons[5].x,
            legacy.buttons[5].y,
            legacy.buttons[5].w,
            legacy.buttons[5].h
        ))
    );
    assert_eq!(
        snap.get("panel_top").map(|e| e.layout.rect),
        plan.rect_of("panel_top")
    );
    let panel_top = plan.rect_of("panel_top").expect("panel_top");
    assert_eq!(
        legacy.panel_top,
        ra_layout::RectPx::new(
            panel_top.x as i32,
            panel_top.y as i32,
            panel_top.width as i32,
            panel_top.height as i32,
        )
    );

    let hit = snap
        .hit_test(Point2 {
            x: legacy.buttons[5].x as f32 + 4.0,
            y: legacy.buttons[5].y as f32 + 4.0,
        })
        .expect("hit resume");
    assert_eq!(hit.id.0, resume);
    assert_eq!(plan.rect_of(resume), Some(hit.layout.rect));
}
