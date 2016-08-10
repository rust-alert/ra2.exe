//! 布局快照。

mod hit;

use crate::{
    geometry::{Point2, Rect},
    spec::LayoutId,
};

/// 单节点布局盒。
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    /// 实际矩形。
    pub rect: Rect,
    /// 裁剪。
    pub clip: Option<Rect>,
    /// 可选文本基线（相对 rect.y）。
    pub baseline: Option<f32>,
}

/// 快照中的元素。
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutElement {
    /// 标识。
    pub id: LayoutId,
    /// 布局盒。
    pub layout: LayoutBox,
    /// 层序。
    pub z_index: i32,
    /// 命中。
    pub hit_region: HitRegion,
    /// 命中模式。
    pub hit_test: HitTestMode,
}

/// 整树布局快照。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayoutSnapshot {
    /// 扁平元素表（绘制与命中同序消费）。
    pub elements: Vec<LayoutElement>,
}

impl LayoutSnapshot {
    /// 按标识查找元素。
    pub fn get(&self, id: &str) -> Option<&LayoutElement> {
        self.elements.iter().find(|e| e.id.0 == id)
    }

    /// 自上而下命中（`z_index` 高者优先；同序时后写入者优先）。
    pub fn hit_test(&self, point: Point2) -> Option<&LayoutElement> {
        self.elements
            .iter()
            .enumerate()
            .filter(|(_, e)| e.hit_test == HitTestMode::Rect)
            .filter(|(_, e)| e.hit_region.rect.contains(point))
            .max_by_key(|(i, e)| (e.z_index, *i))
            .map(|(_, e)| e)
    }
}

pub use hit::{HitRegion, HitTestMode};
