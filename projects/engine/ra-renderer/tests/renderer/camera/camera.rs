//! `ViewCamera` 集成测试（通过 crate 公开重导出访问）。

use ra_renderer::ViewCamera;

#[test]
fn fit_centers_image() {
    let cam = ViewCamera::fit(200, 100, 100, 100);
    assert!((cam.center_x - 100.0).abs() < 0.01);
    assert!((cam.center_y - 50.0).abs() < 0.01);
    assert!((cam.zoom - 0.5).abs() < 0.01);
}

#[test]
fn pan_moves_center() {
    let mut cam = ViewCamera { center_x: 0.0, center_y: 0.0, zoom: 2.0 };
    cam.pan_screen(10.0, -4.0);
    assert!((cam.center_x - (-5.0)).abs() < 0.01);
    assert!((cam.center_y - 2.0).abs() < 0.01);
}

#[test]
fn screen_to_world_inverts_center() {
    let cam = ViewCamera { center_x: 100.0, center_y: 50.0, zoom: 2.0 };
    let (wx, wy) = cam.screen_to_world(400.0, 300.0, 800.0, 600.0);
    assert!((wx - 100.0).abs() < 0.01);
    assert!((wy - 50.0).abs() < 0.01);
}

// 自顶层 `camera_unit.rs` 并入。

// 自 engine/ra-renderer/src/camera.rs :: tests
use ra_renderer::camera::*;

#[test]
fn pan_clamped_stops_at_map_edge() {
    let mut cam = Camera { center_x: 50.0, center_y: 50.0, zoom: 1.0 };
    let bounds = CameraBounds::from_world_and_viewport(100.0, 100.0, 40.0, 40.0, 1.0);
    assert!((bounds.min_center_x - 20.0).abs() < 1e-5);
    assert!((bounds.max_center_x - 80.0).abs() < 1e-5);
    cam.pan_clamped(1000.0, 0.0, &bounds);
    assert!((cam.center_x - 20.0).abs() < 1e-3);
    cam.pan_clamped(-1000.0, 0.0, &bounds);
    assert!((cam.center_x - 80.0).abs() < 1e-3);
}

#[test]
fn smaller_clamp_viewport_than_projection_exposes_void() {
    // 投影按整窗 200，夹紧若误用更窄的 120（如侧栏内 world 视口），
    // 中心可落到 60，整窗可见左缘为 60-100=-40，露出地图外。
    let world_w = 400.0;
    let projection_w = 200.0;
    let hud_world_w = 120.0;
    let zoom = 1.0;
    let loose = CameraBounds::from_world_and_viewport(world_w, world_w, hud_world_w, hud_world_w, zoom);
    let tight = CameraBounds::from_world_and_viewport(world_w, world_w, projection_w, projection_w, zoom);
    assert!(loose.min_center_x < tight.min_center_x);
    let visible_left_if_loose = loose.min_center_x - projection_w * 0.5 / zoom;
    assert!(visible_left_if_loose < 0.0);
    let visible_left_if_tight = tight.min_center_x - projection_w * 0.5 / zoom;
    assert!((visible_left_if_tight - 0.0).abs() < 1e-5);
}

#[test]
fn small_world_locks_center() {
    let mut cam = Camera { center_x: 0.0, center_y: 0.0, zoom: 1.0 };
    let bounds = CameraBounds::from_world_and_viewport(50.0, 50.0, 200.0, 200.0, 1.0);
    assert!(bounds.min_center_x > bounds.max_center_x);
    cam.clamp_to_bounds(&bounds);
    assert!((cam.center_x - 25.0).abs() < 1e-3);
    assert!((cam.center_y - 25.0).abs() < 1e-3);
}

#[test]
fn world_screen_roundtrip() {
    let cam = Camera { center_x: 80.0, center_y: 40.0, zoom: 2.0 };
    let (sx, sy) = cam.world_to_screen(90.0, 50.0, 200.0, 100.0);
    let (wx, wy) = cam.screen_to_world(sx, sy, 200.0, 100.0);
    assert!((wx - 90.0).abs() < 1e-4);
    assert!((wy - 50.0).abs() < 1e-4);
}

#[test]
fn world_screen_roundtrip_in_offset_rect() {
    let cam = Camera { center_x: 80.0, center_y: 40.0, zoom: 2.0 };
    let (ox, oy, vw, vh) = (0.0, 0.0, 160.0, 100.0);
    let (sx, sy) = cam.world_to_screen_in(90.0, 50.0, ox, oy, vw, vh);
    let (wx, wy) = cam.screen_to_world_in(sx, sy, ox, oy, vw, vh);
    assert!((wx - 90.0).abs() < 1e-4);
    assert!((wy - 50.0).abs() < 1e-4);
    // 战术区中心应对准相机中心。
    let (cx, cy) = cam.screen_to_world_in(ox + vw * 0.5, oy + vh * 0.5, ox, oy, vw, vh);
    assert!((cx - 80.0).abs() < 1e-4);
    assert!((cy - 40.0).abs() < 1e-4);
}

#[test]
fn content_rect_inset_hides_outer_void() {
    // LocalSize 投影比整图小：夹紧后可见左缘不再落到预览 0 之外。
    let full = CameraBounds::from_world_and_viewport(400.0, 400.0, 200.0, 200.0, 1.0);
    let local = CameraBounds::from_content_rect(40.0, 40.0, 360.0, 360.0, 200.0, 200.0, 1.0);
    assert!(local.min_center_x > full.min_center_x);
    let visible_left = local.min_center_x - 100.0;
    assert!((visible_left - 40.0).abs() < 1e-5);
}

#[test]
fn matching_proj_and_clamp_keeps_map_edge() {
    // 投影与夹紧同用战术区宽高时，可见左缘贴齐地图，不露出 void。
    let world_w = 400.0;
    let tactical_w = 160.0;
    let zoom = 1.0;
    let bounds = CameraBounds::from_world_and_viewport(world_w, world_w, tactical_w, tactical_w, zoom);
    let visible_left = bounds.min_center_x - tactical_w * 0.5 / zoom;
    assert!((visible_left - 0.0).abs() < 1e-5);
}
