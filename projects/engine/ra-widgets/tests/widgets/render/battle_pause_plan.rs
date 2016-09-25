//! 对局暂停菜单闭环：`solve_battle_pause` → snapshot → hit → `RenderPlan`。

use ra_layout::{BATTLE_PAUSE_MENU_BUTTON_IDS, Point2, solve_battle_pause};
use ra_widgets::RenderPlan;

#[test]
fn battle_pause_render_plan_rects_match_snapshot_hits() {
    let snap = solve_battle_pause();
    let plan = RenderPlan::battle_pause_placeholders();
    let resume = BATTLE_PAUSE_MENU_BUTTON_IDS[3];

    assert_eq!(snap.get(resume).map(|e| e.layout.rect), plan.rect_of(resume));
    assert_eq!(snap.get("dim").map(|e| e.layout.rect), plan.rect_of("dim"));
    assert!(snap.get("panel_top").is_none(), "pause layout must not reuse main-menu chrome");
    assert!(snap.get("sidebar").is_none(), "pause layout must not reuse battle HUD ids");

    let resume_rect = plan.rect_of(resume).expect("resume");
    let hit = snap.hit_test(Point2 { x: resume_rect.x + 4.0, y: resume_rect.y + 4.0 }).expect("hit resume");
    assert_eq!(hit.id.0, resume);
    assert_eq!(plan.rect_of(resume), Some(hit.layout.rect));
}
