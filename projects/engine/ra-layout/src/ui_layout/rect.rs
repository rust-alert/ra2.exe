//! 壳层像素矩形。

use crate::{geometry::Rect, snapshot::LayoutSnapshot};

/// 壳层像素矩形。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectPx {
    /// 左。
    pub x: i32,
    /// 上。
    pub y: i32,
    /// 宽。
    pub w: i32,
    /// 高。
    pub h: i32,
}

impl RectPx {
    /// 构造。
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// 是否包含点（像素，半开区间右下）。
    pub fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }
}

/// 从 `LayoutSnapshot` 取整数像素矩形（缺省为零矩形）。
pub(super) fn rect_px_from_snapshot(snap: &LayoutSnapshot, id: &str) -> RectPx {
    let Rect {
        x,
        y,
        width,
        height,
    } = snap.get(id).map(|e| e.layout.rect).unwrap_or_default();
    RectPx::new(x as i32, y as i32, width as i32, height as i32)
}
