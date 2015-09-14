//! 对局 HUD 闭环：`solve_battle_hud` → snapshot → hit → `RenderPlan`。

use ra_layout::{battle_hud_layout, solve_battle_hud, Point2};
use ra_widgets::RenderPlan;

#[test]
fn battle_hud_render_plan_rects_match_snapshot_hits() {
    let (vw, vh) = (1280u32, 720u32);
    let snap = solve_battle_hud(vw, vh);
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

    let world = legacy.world_viewport();
    assert_eq!(world.x, 0);
    assert_eq!(world.y, 0);
    assert_eq!(world.w, legacy.sidebar.x);
    assert_eq!(world.h, legacy.command_bar.y, "tactical area stops above command bar");
    assert_eq!(legacy.command_bar.h, ra_layout::COMMAND_BAR_H);
    assert_eq!(legacy.command_bar.x, 0);
    assert_eq!(legacy.command_bar.w, legacy.sidebar.x);
    assert_eq!(legacy.bottom_strip.x, legacy.sidebar.x);
    assert_eq!(legacy.bottom_strip.w, legacy.sidebar.w);
    // 选项/外交命中格必须在右栏内，不能落到战术区。
    assert!(legacy.opt_btn.x >= legacy.sidebar.x);
    assert!(legacy.diplo_btn.x >= legacy.sidebar.x);
    assert!(legacy.repair.y >= legacy.side1.y);
    assert!(legacy.sell.y >= legacy.side1.y);
}
