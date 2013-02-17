//! 2D 相机：平移 + 缩放，把世界像素投到 NDC。

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
    pub const ZOOM_MIN: f32 = 0.05;
    pub const ZOOM_MAX: f32 = 8.0;

    /// 以图像中心为焦点，缩放到刚好塞进窗口。
    pub fn fit(image_w: u32, image_h: u32, screen_w: u32, screen_h: u32) -> Self {
        let iw = image_w.max(1) as f32;
        let ih = image_h.max(1) as f32;
        let sw = screen_w.max(1) as f32;
        let sh = screen_h.max(1) as f32;
        let zoom = (sw / iw).min(sh / ih).clamp(Self::ZOOM_MIN, Self::ZOOM_MAX);
        Self {
            center_x: iw * 0.5,
            center_y: ih * 0.5,
            zoom,
        }
    }

    /// 屏幕位移（像素）转世界平移；拖拽向右时地图跟随。
    pub fn pan_screen(&mut self, dx_screen: f32, dy_screen: f32) {
        let z = self.zoom.max(0.0001);
        self.center_x -= dx_screen / z;
        self.center_y -= dy_screen / z;
    }

    /// 相对缩放，夹在合理范围。
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(Self::ZOOM_MIN, Self::ZOOM_MAX);
    }

    /// 世界像素 → 裁剪空间（NDC，Y 向上）。
    pub fn world_to_ndc(&self, wx: f32, wy: f32, screen_w: f32, screen_h: f32) -> [f32; 2] {
        let sw = screen_w.max(1.0);
        let sh = screen_h.max(1.0);
        let sx = (wx - self.center_x) * self.zoom + sw * 0.5;
        let sy = (wy - self.center_y) * self.zoom + sh * 0.5;
        let ndc_x = sx / sw * 2.0 - 1.0;
        let ndc_y = 1.0 - sy / sh * 2.0;
        [ndc_x, ndc_y]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_centers_image() {
        let cam = Camera::fit(200, 100, 100, 100);
        assert!((cam.center_x - 100.0).abs() < 0.01);
        assert!((cam.center_y - 50.0).abs() < 0.01);
        assert!((cam.zoom - 0.5).abs() < 0.01);
    }

    #[test]
    fn pan_moves_center() {
        let mut cam = Camera {
            center_x: 0.0,
            center_y: 0.0,
            zoom: 2.0,
        };
        cam.pan_screen(10.0, -4.0);
        assert!((cam.center_x - (-5.0)).abs() < 0.01);
        assert!((cam.center_y - 2.0).abs() < 0.01);
    }
}
