//! 普通几何结果（逻辑像素，非原版低分辨率参考表）。

/// 二维点（逻辑像素）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point2 {
    /// X。
    pub x: f32,
    /// Y。
    pub y: f32,
}

/// 二维尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size2 {
    /// 宽。
    pub width: f32,
    /// 高。
    pub height: f32,
}

/// 轴对齐矩形。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// 左。
    pub x: f32,
    /// 上。
    pub y: f32,
    /// 宽。
    pub width: f32,
    /// 高。
    pub height: f32,
}

impl Rect {
    /// 由原点与尺寸构造。
    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// 点是否落在矩形内（含左上、不含右下边界约定：`[min, max)`）。
    pub fn contains(&self, point: Point2) -> bool {
        point.x >= self.x && point.y >= self.y && point.x < self.x + self.width && point.y < self.y + self.height
    }

    /// 归一化框 `[x0,y0)…(x1,y1]` → 设计像素矩形（相对画布宽高）。
    pub fn from_frac(x0: f32, y0: f32, x1: f32, y1: f32, canvas_w: f32, canvas_h: f32) -> Self {
        Self::from_xywh(
            (x0 * canvas_w).round(),
            (y0 * canvas_h).round(),
            ((x1 - x0) * canvas_w).round().max(1.0),
            ((y1 - y0) * canvas_h).round().max(1.0),
        )
    }
}

/// 四边内边距 / 外边距。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Insets {
    /// 左。
    pub left: f32,
    /// 上。
    pub top: f32,
    /// 右。
    pub right: f32,
    /// 下。
    pub bottom: f32,
}
