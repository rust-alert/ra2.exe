//! 对局地图视口契约：战术区命中、投影与 world pass 裁切共用同一矩形。

use ra_renderer::{CameraBounds, ViewCamera};

use crate::{battle_hud_world_viewport, solve_battle_hud};

use super::RectPx;

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
    /// 由窗口像素构造：战术区取自 `solve_battle_hud` → `battle_hud_world_viewport`。
    pub fn battle(window_w: u32, window_h: u32) -> Self {
        let sw = window_w.max(1);
        let sh = window_h.max(1);
        let snap = solve_battle_hud(sw, sh);
        let tactical = battle_hud_world_viewport(&snap);
        Self { tactical, surface_w: sw, surface_h: sh }
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
        cam.screen_to_world_in(sx, sy, self.tactical.x as f32, self.tactical.y as f32, self.proj_w(), self.proj_h())
    }

    /// 世界像素 → 窗口像素。
    pub fn world_to_screen(&self, cam: &ViewCamera, wx: f32, wy: f32) -> (f32, f32) {
        cam.world_to_screen_in(wx, wy, self.tactical.x as f32, self.tactical.y as f32, self.proj_w(), self.proj_h())
    }

    /// 与战术区投影同口径的相机中心夹紧范围。
    pub fn camera_bounds(&self, world_w: f32, world_h: f32, zoom: f32) -> CameraBounds {
        CameraBounds::from_world_and_viewport(world_w, world_h, self.proj_w(), self.proj_h(), zoom)
    }

    /// wgpu `set_viewport` / `set_scissor_rect` 用的像素矩形 `(x, y, w, h)`。
    pub fn clip_rect_u32(&self) -> (u32, u32, u32, u32) {
        (self.tactical.x.max(0) as u32, self.tactical.y.max(0) as u32, self.tactical.w.max(0) as u32, self.tactical.h.max(0) as u32)
    }
}
