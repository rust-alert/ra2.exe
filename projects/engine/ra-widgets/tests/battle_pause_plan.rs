//! 对局暂停菜单闭环：`shell_page_layout_tree` → snapshot → hit → `RenderPlan`。

use ra_layout::{
    battle_pause_menu_layout, shell_design_size, shell_page_layout_tree, LayoutEngine,
    BATTLE_PAUSE_MENU_BUTTON_IDS, Point2, RightPanelChrome, Viewport,
};
use ra_widgets::RenderPlan;

#[test]
fn battle_pause_render_plan_rects_match_snapshot_hits() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: shell_design_size(chrome),
            ..Viewport::default()
        },
        &shell_page_layout_tree(
            "battle_pause",
            &BATTLE_PAUSE_MENU_BUTTON_IDS[..5],
            Some(BATTLE_PAUSE_MENU_BUTTON_IDS[5]),
            chrome,
        ),
    );
    let plan = RenderPlan::shell_page_placeholders(
        "battle_pause",
        &BATTLE_PAUSE_MENU_BUTTON_IDS[..5],
        Some(BATTLE_PAUSE_MENU_BUTTON_IDS[5]),
    );
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

    let hit = snap
        .hit_test(Point2 {
            x: legacy.buttons[5].x as f32 + 4.0,
            y: legacy.buttons[5].y as f32 + 4.0,
        })
        .expect("hit resume");
    assert_eq!(hit.id.0, resume);
    assert_eq!(plan.rect_of(resume), Some(hit.layout.rect));
}
