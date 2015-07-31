//! 2D 相机：平移 + 缩放，把世界像素投到 NDC。

/// 相机中心在世界像素中的可移动范围。
///
/// 当世界小于可视区域时 `min_* > max_*`，夹紧会落到中点，整图居中且不产生可拖黑边。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraBounds {
    /// 中心 X 下限。
    pub min_center_x: f32,
    /// 中心 X 上限。
    pub max_center_x: f32,
    /// 中心 Y 下限。
    pub min_center_y: f32,
    /// 中心 Y 上限。
    pub max_center_y: f32,
}

impl CameraBounds {
    /// 由世界矩形 `[0, world_w] × [0, world_h]` 与可视 viewport（屏幕像素）计算中心夹紧范围。
    pub fn from_world_and_viewport(world_w: f32, world_h: f32, viewport_w: f32, viewport_h: f32, zoom: f32) -> Self {
        let z = zoom.max(0.0001);
        let half_w = viewport_w.max(1.0) * 0.5 / z;
        let half_h = viewport_h.max(1.0) * 0.5 / z;
        Self {
            min_center_x: half_w,
            max_center_x: world_w - half_w,
            min_center_y: half_h,
            max_center_y: world_h - half_h,
        }
    }
}

/// 2D 视口相机：以世界像素为坐标，经缩放映射到屏幕与 NDC。
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// 视口中心对应的世界 X（图像像素）。
    pub center_x: f32,
    /// 视口中心对应的世界 Y（图像像素，向下为正）。
    pub center_y: f32,
    /// 屏幕像素 / 世界像素。
    pub zoom: f32,
}

impl Camera {
    /// 允许的最小缩放比（屏幕像素 / 世界像素）。
    pub const ZOOM_MIN: f32 = 0.05;
    /// 允许的最大缩放比（屏幕像素 / 世界像素）。
    pub const ZOOM_MAX: f32 = 8.0;

    /// 以图像中心为焦点，缩放到刚好塞进窗口。
    pub fn fit(image_w: u32, image_h: u32, screen_w: u32, screen_h: u32) -> Self {
        let iw = image_w.max(1) as f32;
        let ih = image_h.max(1) as f32;
        let sw = screen_w.max(1) as f32;
        let sh = screen_h.max(1) as f32;
        let zoom = (sw / iw).min(sh / ih).clamp(Self::ZOOM_MIN, Self::ZOOM_MAX);
        let mut cam = Self { center_x: iw * 0.5, center_y: ih * 0.5, zoom };
        let bounds = CameraBounds::from_world_and_viewport(iw, ih, sw, sh, cam.zoom);
        cam.clamp_to_bounds(&bounds);
        cam
    }

    /// 屏幕位移（像素）转世界平移；拖拽向右时地图跟随。
    pub fn pan_screen(&mut self, dx_screen: f32, dy_screen: f32) {
        let z = self.zoom.max(0.0001);
        self.center_x -= dx_screen / z;
        self.center_y -= dy_screen / z;
    }

    /// 平移后按边界夹紧（地图小于视口时锁中心）。
    pub fn pan_clamped(&mut self, dx_screen: f32, dy_screen: f32, bounds: &CameraBounds) {
        self.pan_screen(dx_screen, dy_screen);
        self.clamp_to_bounds(bounds);
    }

    /// 将中心夹到边界内。
    pub fn clamp_to_bounds(&mut self, bounds: &CameraBounds) {
        self.center_x = clamp_axis(self.center_x, bounds.min_center_x, bounds.max_center_x);
        self.center_y = clamp_axis(self.center_y, bounds.min_center_y, bounds.max_center_y);
    }

    /// 相对缩放，夹在合理范围。
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(Self::ZOOM_MIN, Self::ZOOM_MAX);
    }

    /// 屏幕像素 → 世界（预览图像素）；投影原点为 `(0,0)`、尺寸为整窗。
    pub fn screen_to_world(&self, sx: f32, sy: f32, screen_w: f32, screen_h: f32) -> (f32, f32) {
        self.screen_to_world_in(sx, sy, 0.0, 0.0, screen_w, screen_h)
    }

    /// 世界像素 → 屏幕像素；投影原点为 `(0,0)`、尺寸为整窗。
    pub fn world_to_screen(&self, wx: f32, wy: f32, screen_w: f32, screen_h: f32) -> (f32, f32) {
        self.world_to_screen_in(wx, wy, 0.0, 0.0, screen_w, screen_h)
    }

    /// 屏幕像素 → 世界，投影中心为矩形 `(ox,oy,vw,vh)` 的几何中心。
    ///
    /// 对局战术区与整窗投影必须共用此口径，避免侧栏偏移造成命中漂移。
    pub fn screen_to_world_in(&self, sx: f32, sy: f32, ox: f32, oy: f32, vw: f32, vh: f32) -> (f32, f32) {
        let z = self.zoom.max(0.0001);
        let vw = vw.max(1.0);
        let vh = vh.max(1.0);
        let wx = (sx - ox - vw * 0.5) / z + self.center_x;
        let wy = (sy - oy - vh * 0.5) / z + self.center_y;
        (wx, wy)
    }

    /// 世界像素 → 屏幕像素，投影中心为矩形 `(ox,oy,vw,vh)` 的几何中心。
    pub fn world_to_screen_in(&self, wx: f32, wy: f32, ox: f32, oy: f32, vw: f32, vh: f32) -> (f32, f32) {
        let vw = vw.max(1.0);
        let vh = vh.max(1.0);
        let sx = (wx - self.center_x) * self.zoom + ox + vw * 0.5;
        let sy = (wy - self.center_y) * self.zoom + oy + vh * 0.5;
        (sx, sy)
    }

    /// 世界像素 → 裁剪空间（NDC，Y 向上）。
    ///
    /// `screen_w`/`screen_h` 为**投影矩形**尺寸。若 GPU pass 已 `set_viewport` 到同一矩形，
    /// NDC `[-1,1]` 会映射到该 viewport，无需再乘表面偏移。
    pub fn world_to_ndc(&self, wx: f32, wy: f32, screen_w: f32, screen_h: f32) -> [f32; 2] {
        let sw = screen_w.max(1.0);
        let sh = screen_h.max(1.0);
        let (sx, sy) = self.world_to_screen(wx, wy, sw, sh);
        let ndc_x = sx / sw * 2.0 - 1.0;
        let ndc_y = 1.0 - sy / sh * 2.0;
        [ndc_x, ndc_y]
    }
}

fn clamp_axis(value: f32, min: f32, max: f32) -> f32 {
    if min > max {
        (min + max) * 0.5
    } else {
        value.clamp(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn matching_proj_and_clamp_keeps_map_edge() {
        // 投影与夹紧同用战术区宽高时，可见左缘贴齐地图，不露出 void。
        let world_w = 400.0;
        let tactical_w = 160.0;
        let zoom = 1.0;
        let bounds = CameraBounds::from_world_and_viewport(world_w, world_w, tactical_w, tactical_w, zoom);
        let visible_left = bounds.min_center_x - tactical_w * 0.5 / zoom;
        assert!((visible_left - 0.0).abs() < 1e-5);
    }
}
