//! 主菜单闭环：`solve_shell_page` → snapshot → hit → `RenderPlan`。

use ra_layout::{main_menu_layout, solve_shell_page, MAIN_MENU_BUTTON_IDS, Point2};
use ra_widgets::RenderPlan;

#[test]
fn main_menu_render_plan_rects_match_snapshot_hits() {
    let snap = solve_shell_page(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    let plan = RenderPlan::shell_page_placeholders(
        "main_menu",
        &MAIN_MENU_BUTTON_IDS[..5],
        Some(MAIN_MENU_BUTTON_IDS[5]),
    );
    let legacy = main_menu_layout(800, 600);

    assert_eq!(
        plan.rect_of("exit").map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((
            legacy.buttons[5].x,
            legacy.buttons[5].y,
            legacy.buttons[5].w,
            legacy.buttons[5].h
        ))
    );
    assert_eq!(
        snap.get("title").map(|e| e.layout.rect),
        plan.rect_of("title")
    );

    let hit = snap
        .hit_test(Point2 {
            x: legacy.buttons[0].x as f32 + 4.0,
            y: legacy.buttons[0].y as f32 + 4.0,
        })
        .expect("hit single_player");
    assert_eq!(hit.id.0, "single_player");
    assert_eq!(plan.rect_of("single_player"), Some(hit.layout.rect));
}
