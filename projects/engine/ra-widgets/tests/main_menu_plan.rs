//! 主菜单闭环：`shell_page_layout_tree` → snapshot → hit → `RenderPlan`。

use ra_layout::{
    main_menu_layout, shell_design_size, shell_page_layout_tree, LayoutEngine, MAIN_MENU_BUTTON_IDS,
    Point2, RightPanelChrome, Viewport,
};
use ra_widgets::RenderPlan;

#[test]
fn main_menu_render_plan_rects_match_snapshot_hits() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(
        Viewport {
            size: shell_design_size(chrome),
            ..Viewport::default()
        },
        &shell_page_layout_tree(
            "main_menu",
            &MAIN_MENU_BUTTON_IDS[..5],
            Some(MAIN_MENU_BUTTON_IDS[5]),
            chrome,
        ),
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
