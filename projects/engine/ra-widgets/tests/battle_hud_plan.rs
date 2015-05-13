//! 对局 HUD 闭环：layout tree → snapshot → hit → `RenderPlan`。

use ra_layout::{
    battle_hud_layout, battle_hud_layout_tree, LayoutEngine, Point2, Size2, Viewport,
};
use ra_widgets::RenderPlan;

#[test]
fn battle_hud_render_plan_rects_match_snapshot_hits() {
    let (vw, vh) = (1280u32, 720u32);
    let snap = LayoutEngine.solve(
        Viewport {
            size: Size2 {
                width: vw as f32,
                height: vh as f32,
            },
            ..Viewport::default()
        },
        &battle_hud_layout_tree(vw, vh),
    );
    let plan = RenderPlan::battle_hud_placeholders(vw, vh);
    let legacy = battle_hud_layout(vw, vh);

    assert_eq!(
        plan.rect_of("opt_btn").map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((legacy.opt_btn.x, legacy.opt_btn.y, legacy.opt_btn.w, legacy.opt_btn.h))
    );
    assert_eq!(
        snap.get("repair").map(|e| e.layout.rect),
        plan.rect_of("repair")
    );

    let hit = snap
        .hit_test(Point2 {
            x: legacy.opt_btn.x as f32 + 4.0,
            y: legacy.opt_btn.y as f32 + 4.0,
        })
        .expect("hit opt_btn");
    assert_eq!(hit.id.0, "opt_btn");
    assert_eq!(plan.rect_of("opt_btn"), Some(hit.layout.rect));
}
