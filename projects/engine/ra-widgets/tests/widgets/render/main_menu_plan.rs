//! 主菜单闭环：`solve_shell_page` → snapshot → hit → `RenderPlan`。

use ra_layout::{MAIN_MENU_BUTTON_IDS, Point2, solve_shell_page};
use ra_widgets::RenderPlan;

#[test]
fn main_menu_render_plan_rects_match_snapshot_hits() {
    let snap = solve_shell_page("main_menu", &MAIN_MENU_BUTTON_IDS[..5], Some(MAIN_MENU_BUTTON_IDS[5]));
    let plan = RenderPlan::shell_page_placeholders("main_menu", &MAIN_MENU_BUTTON_IDS[..5], Some(MAIN_MENU_BUTTON_IDS[5]));
    let exit = MAIN_MENU_BUTTON_IDS[5];

    assert_eq!(snap.get(exit).map(|e| e.layout.rect), plan.rect_of(exit));
    assert_eq!(snap.get("panel_top").map(|e| e.layout.rect), plan.rect_of("panel_top"));

    let exit_rect = plan.rect_of(exit).expect("exit");
    let hit = snap.hit_test(Point2 { x: exit_rect.x + 4.0, y: exit_rect.y + 4.0 }).expect("hit exit");
    assert_eq!(hit.id.0, exit);
    assert_eq!(plan.rect_of(exit), Some(hit.layout.rect));
}
