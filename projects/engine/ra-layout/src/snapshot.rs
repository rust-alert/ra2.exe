// reshape-layout-components:skeleton
//! 布局求解结果（绘制与命中共用）。

use crate::{
    geometry::Rect,
    hit::{HitRegion, HitTestMode},
    node::LayoutId,
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
