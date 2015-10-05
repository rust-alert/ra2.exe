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

/// 壳层 800×600 内容在窗口中的 letterbox 目标矩形（与 [`shell_fit_camera`] 一致）。
pub fn shell_content_rect_in_window(win_w: u32, win_h: u32) -> RectPx {
    let w = win_w.max(1);
    let h = win_h.max(1);
    let cam = shell_fit_camera(w, h);
    let (x0, y0) = cam.world_to_screen(0.0, 0.0, w as f32, h as f32);
    let (x1, y1) = cam.world_to_screen(SHELL_BASE_W as f32, SHELL_BASE_H as f32, w as f32, h as f32);
    // 用 round 稳定 letterbox 边，避免 fit 缩放浮点落在整数下方时 floor 偏一像素。
    let x = x0.round() as i32;
    let y = y0.round() as i32;
    let rw = (x1 - x0).round().max(1.0) as i32;
    let rh = (y1 - y0).round().max(1.0) as i32;
    RectPx::new(x, y, rw, rh)
}
