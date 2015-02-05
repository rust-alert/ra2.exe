//! 窗口坐标与壳层设计坐标映射。

use ra_renderer::ViewCamera;
use super::*;


/// 与 UI 页上传后相同的 fit 相机（内容 800×600 → 窗口）。
pub fn shell_fit_camera(win_w: u32, win_h: u32) -> ViewCamera {
    ViewCamera::fit(SHELL_BASE_W as u32, SHELL_BASE_H as u32, win_w.max(1), win_h.max(1))
}

/// 窗口像素 → 壳层内容像素（与 [`shell_fit_camera`] 一致）。
pub fn window_to_shell_px(cursor_x: f64, cursor_y: f64, win_w: f64, win_h: f64) -> (i32, i32) {
    let cam = shell_fit_camera(win_w.max(1.0) as u32, win_h.max(1.0) as u32);
    let (wx, wy) = cam.screen_to_world(cursor_x as f32, cursor_y as f32, win_w.max(1.0) as f32, win_h.max(1.0) as f32);
    (wx.floor() as i32, wy.floor() as i32)
}
