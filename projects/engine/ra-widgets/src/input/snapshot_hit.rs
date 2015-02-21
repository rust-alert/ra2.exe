//! 从 `LayoutSnapshot` 命中（绘制与命中同源）。

use ra_layout::{LayoutElement, LayoutSnapshot, Point2};

/// 在设计坐标下命中快照元素。
pub fn hit_element_at<'a>(snapshot: &'a LayoutSnapshot, x: f32, y: f32) -> Option<&'a LayoutElement> {
    snapshot.hit_test(Point2 { x, y })
}

/// 命中控件 id（字符串标签）。
pub fn hit_id_at(snapshot: &LayoutSnapshot, x: f32, y: f32) -> Option<&str> {
    hit_element_at(snapshot, x, y).map(|e| e.id.0.as_str())
}
