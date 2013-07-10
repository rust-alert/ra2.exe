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
