//! 由已解析设计矩形 / 流式容器组装 `LayoutNode` 树。

use crate::{
    geometry::{Rect, Size2},
    policy::{HorizontalRule, LayoutFlow, LayoutRules, SizeRule, VerticalRule},
    spec::{LayoutId, LayoutNode},
};

/// 根节点铺满设计视口，子节点为绝对定位叶子。
pub fn root_with_fixed_children(root_id: impl Into<String>, _design: Size2, children: Vec<LayoutNode>) -> LayoutNode {
    root_with_children(root_id, children)
}

/// 根节点铺满父盒（通常为视口）。
pub fn root_with_children(root_id: impl Into<String>, children: Vec<LayoutNode>) -> LayoutNode {
    LayoutNode {
        id: LayoutId(root_id.into()),
        rules: LayoutRules { horizontal: HorizontalRule::Stretch, vertical: VerticalRule::Stretch, ..LayoutRules::default() },
        children,
    }
}

/// 固定设计像素矩形的叶子（相对父盒 `Start`/`Top`）。
pub fn fixed_rect_leaf(id: impl Into<String>, rect: Rect) -> LayoutNode {
    LayoutNode::leaf(
        id,
        LayoutRules {
            horizontal: HorizontalRule::Start(rect.x),
            vertical: VerticalRule::Top(rect.y),
            width: SizeRule::Fixed(rect.width),
            height: SizeRule::Fixed(rect.height),
            ..LayoutRules::default()
        },
    )
}

/// 固有宽高叶子（流式子项；相对父盒从原点起算，由 Row/Column 放置）。
pub fn sized_leaf(id: impl Into<String>, width: f32, height: f32) -> LayoutNode {
    LayoutNode::leaf(
        id,
        LayoutRules { width: SizeRule::Fixed(width.max(0.0)), height: SizeRule::Fixed(height.max(0.0)), ..LayoutRules::default() },
    )
}

/// 纵向流式容器：固有尺寸由子树决定，不参与命中。
pub fn column(id: impl Into<String>, gap: f32, children: Vec<LayoutNode>) -> LayoutNode {
    LayoutNode { id: LayoutId(id.into()), rules: LayoutRules::flow_container(LayoutFlow::Column { gap: gap.max(0.0) }), children }
}

/// 横向流式容器：固有尺寸由子树决定，不参与命中。
pub fn row(id: impl Into<String>, gap: f32, children: Vec<LayoutNode>) -> LayoutNode {
    LayoutNode { id: LayoutId(id.into()), rules: LayoutRules::flow_container(LayoutFlow::Row { gap: gap.max(0.0) }), children }
}
