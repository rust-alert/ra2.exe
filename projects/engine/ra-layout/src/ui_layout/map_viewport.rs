//! 对局地图视口契约：战术区命中、投影与 world pass 裁切共用同一矩形。

use ra_renderer::{CameraBounds, ViewCamera};

use super::{battle_hud_layout, RectPx};

/// 对局地图视口：世界绘制、marker、命中与相机边界必须使用同一实例。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapViewport {
    /// 战术区屏幕矩形（左起至侧栏左缘，下至命令条顶边）。
    pub tactical: RectPx,
    /// 整窗表面宽（HUD 等非世界 pass 仍按整窗）。
    pub surface_w: u32,
    /// 整窗表面高。
    pub surface_h: u32,
}

impl MapViewport {
    /// 由窗口像素构造：战术区取自 [`battle_hud_layout`] 的 `world_viewport`。
    pub fn battle(window_w: u32, window_h: u32) -> Self {
        let sw = window_w.max(1);
        let sh = window_h.max(1);
        let tactical = battle_hud_layout(sw, sh).world_viewport();
        Self {
            tactical,
            surface_w: sw,
            surface_h: sh,
        }
    }

    /// 投影宽（战术区），供 `Camera` / `CameraBounds` 使用。
    pub fn proj_w(&self) -> f32 {
        self.tactical.w.max(1) as f32
    }

    /// 投影高（战术区）。
    pub fn proj_h(&self) -> f32 {
        self.tactical.h.max(1) as f32
    }

    /// 光标是否落在战术区内。
    pub fn contains_cursor(&self, x: i32, y: i32) -> bool {
        self.tactical.w > 0 && self.tactical.h > 0 && self.tactical.contains(x, y)
    }

    /// 窗口像素 → 世界（预览图像素）。
    pub fn screen_to_world(&self, cam: &ViewCamera, sx: f32, sy: f32) -> (f32, f32) {
        cam.screen_to_world_in(
            sx,
            sy,
            self.tactical.x as f32,
            self.tactical.y as f32,
            self.proj_w(),
            self.proj_h(),
        )
    }

    /// 世界像素 → 窗口像素。
    pub fn world_to_screen(&self, cam: &ViewCamera, wx: f32, wy: f32) -> (f32, f32) {
        cam.world_to_screen_in(
            wx,
            wy,
            self.tactical.x as f32,
            self.tactical.y as f32,
            self.proj_w(),
            self.proj_h(),
        )
    }

    /// 与战术区投影同口径的相机中心夹紧范围。
    pub fn camera_bounds(&self, world_w: f32, world_h: f32, zoom: f32) -> CameraBounds {
        CameraBounds::from_world_and_viewport(world_w, world_h, self.proj_w(), self.proj_h(), zoom)
    }

    /// wgpu `set_viewport` / `set_scissor_rect` 用的像素矩形 `(x, y, w, h)`。
    pub fn clip_rect_u32(&self) -> (u32, u32, u32, u32) {
        (
            self.tactical.x.max(0) as u32,
            self.tactical.y.max(0) as u32,
            self.tactical.w.max(0) as u32,
            self.tactical.h.max(0) as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_renderer::ViewCamera;

    #[test]
    fn battle_tactical_matches_hud_world_viewport() {
        let vp = MapViewport::battle(1280, 720);
        let legacy = battle_hud_layout(1280, 720).world_viewport();
        assert_eq!(vp.tactical, legacy);
        assert_eq!(vp.surface_w, 1280);
        assert_eq!(vp.surface_h, 720);
        assert!(vp.proj_w() < 1280.0);
        assert!((vp.proj_h() - (720.0 - crate::COMMAND_BAR_H as f32)).abs() < 1e-3);
    }

    #[test]
    fn screen_world_roundtrip_uses_tactical_center() {
        let vp = MapViewport::battle(1280, 720);
        let cam = ViewCamera {
            center_x: 400.0,
            center_y: 300.0,
            zoom: 1.0,
        };
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
        let cam = ViewCamera {
            center_x: 400.0,
            center_y: 300.0,
            zoom: 1.0,
        };
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
}
