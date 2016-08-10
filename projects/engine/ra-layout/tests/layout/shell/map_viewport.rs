//! 自 `engine/ra-layout/src/shell/map_viewport.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/shell/map_viewport.rs :: tests
use ra_layout::{battle_hud_world_viewport, shell::map_viewport::*, solve_battle_hud};
use ra_renderer::{CameraBounds, ViewCamera};

#[test]
fn battle_tactical_matches_hud_world_viewport() {
    let vp = MapViewport::battle(1280, 720);
    let snap = solve_battle_hud(1280, 720);
    let world = battle_hud_world_viewport(&snap);
    assert_eq!(vp.tactical, world);
    assert_eq!(vp.surface_w, 1280);
    assert_eq!(vp.surface_h, 720);
    assert!(vp.proj_w() < 1280.0);
    assert!((vp.proj_h() - (720.0 - ra_layout::COMMAND_BAR_H as f32)).abs() < 1e-3);
}

#[test]
fn screen_world_roundtrip_uses_tactical_center() {
    let vp = MapViewport::battle(1280, 720);
    let cam = ViewCamera { center_x: 400.0, center_y: 300.0, zoom: 1.0 };
    let cx = vp.tactical.x as f32 + vp.proj_w() * 0.5;
    let cy = vp.tactical.y as f32 + vp.proj_h() * 0.5;
    let (wx, wy) = vp.screen_to_world(&cam, cx, cy);
    assert!((wx - 400.0).abs() < 1e-3);
    assert!((wy - 300.0).abs() < 1e-3);
    let (sx, sy) = vp.world_to_screen(&cam, 400.0, 300.0);
    assert!((sx - cx).abs() < 1e-3);
    assert!((sy - cy).abs() < 1e-3);
}

#[test]
fn full_window_center_is_not_tactical_center() {
    // 整窗中心落在侧栏内或偏右时，不得再当作地图投影中心。
    let vp = MapViewport::battle(1280, 720);
    let cam = ViewCamera { center_x: 400.0, center_y: 300.0, zoom: 1.0 };
    let full_cx = 1280.0 * 0.5;
    let full_cy = 720.0 * 0.5;
    let (wx_full, _) = cam.screen_to_world(full_cx, full_cy, 1280.0, 720.0);
    let (wx_tac, _) = vp.screen_to_world(&cam, full_cx, full_cy);
    assert!((wx_full - 400.0).abs() < 1e-3);
    assert!((wx_tac - wx_full).abs() > 1.0, "sidebar offset must shift hit X");
}

#[test]
fn camera_bounds_match_tactical_proj() {
    let vp = MapViewport::battle(1280, 720);
    let bounds = vp.camera_bounds(2000.0, 2000.0, 1.0);
    let expected = CameraBounds::from_world_and_viewport(2000.0, 2000.0, vp.proj_w(), vp.proj_h(), 1.0);
    assert_eq!(bounds, expected);
    let visible_left = bounds.min_center_x - vp.proj_w() * 0.5;
    assert!(visible_left >= -1e-3);
}
