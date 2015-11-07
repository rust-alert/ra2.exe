//! 对局 HUD 闭环：`solve_battle_hud` → snapshot → hit → `RenderPlan`。

use ra_layout::{
    battle_hud_world_viewport, rect_px_from_snapshot, solve_battle_hud, Point2, COMMAND_BAR_BUTTON_IDS,
    COMMAND_BAR_H, COMMAND_BUTTON_W, COMMAND_LENDCAP_W,
};
use ra_widgets::RenderPlan;

#[test]
fn battle_hud_render_plan_rects_match_snapshot_hits() {
    let (vw, vh) = (1280u32, 720u32);
    let snap = solve_battle_hud(vw, vh);
    let plan = RenderPlan::battle_hud_placeholders(vw, vh);
    let opt_btn = rect_px_from_snapshot(&snap, "opt_btn");
    let sidebar = rect_px_from_snapshot(&snap, "sidebar");
    let command_bar = rect_px_from_snapshot(&snap, "command_bar");
    let bottom_strip = rect_px_from_snapshot(&snap, "bottom_strip");
    let diplo_btn = rect_px_from_snapshot(&snap, "diplo_btn");
    let repair = rect_px_from_snapshot(&snap, "repair");
    let sell = rect_px_from_snapshot(&snap, "sell");
    let side1 = rect_px_from_snapshot(&snap, "side1");
    let cmd0 = rect_px_from_snapshot(&snap, "cmd0");
    let lendcap = rect_px_from_snapshot(&snap, "lendcap");

    assert_eq!(
        plan.rect_of("opt_btn").map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32)),
        Some((opt_btn.x, opt_btn.y, opt_btn.w, opt_btn.h))
    );
    assert_eq!(
        snap.get("repair").map(|e| e.layout.rect),
        plan.rect_of("repair")
    );
    assert_eq!(plan.rect_of("cmd0"), snap.get("cmd0").map(|e| e.layout.rect));
    assert_eq!(lendcap.w, COMMAND_LENDCAP_W);
    assert_eq!(cmd0.x, COMMAND_LENDCAP_W);
    assert_eq!(cmd0.w, COMMAND_BUTTON_W);
    assert_eq!(COMMAND_BAR_BUTTON_IDS.len(), 6);

    let hit = snap
        .hit_test(Point2 {
            x: opt_btn.x as f32 + 4.0,
            y: opt_btn.y as f32 + 4.0,
        })
        .expect("hit opt_btn");
    assert_eq!(hit.id.0, "opt_btn");
    assert_eq!(plan.rect_of("opt_btn"), Some(hit.layout.rect));

    let cmd_hit = snap
        .hit_test(Point2 {
            x: cmd0.x as f32 + 4.0,
            y: cmd0.y as f32 + 4.0,
        })
        .expect("hit cmd0");
    assert_eq!(cmd_hit.id.0, "cmd0");

    let world = battle_hud_world_viewport(&snap);
    assert_eq!(world.x, 0);
    assert_eq!(world.y, 0);
    assert_eq!(world.w, sidebar.x);
    assert_eq!(world.h, command_bar.y, "tactical area stops above command bar");
    assert_eq!(command_bar.h, COMMAND_BAR_H);
    assert_eq!(command_bar.x, 0);
    assert_eq!(command_bar.w, sidebar.x);
    assert_eq!(bottom_strip.x, sidebar.x);
    assert_eq!(bottom_strip.w, sidebar.w);
    // 选项/外交命中格必须在右栏内，不能落到战术区。
    assert!(opt_btn.x >= sidebar.x);
    assert!(diplo_btn.x >= sidebar.x);
    assert!(repair.y >= side1.y);
    assert!(sell.y >= side1.y);
}
