//! 由已解析设计矩形组装 `LayoutNode` 树。

use crate::{
    geometry::{Rect, Size2},
    policy::{HorizontalRule, LayoutRules, SizeRule, VerticalRule},
    spec::{LayoutId, LayoutNode},
};

/// 根节点铺满设计视口，子节点为绝对定位叶子。
pub fn root_with_fixed_children(
    root_id: impl Into<String>,
    _design: Size2,
    children: Vec<LayoutNode>,
) -> LayoutNode {
    LayoutNode {
        id: LayoutId(root_id.into()),
        rules: LayoutRules {
            horizontal: HorizontalRule::Stretch,
            vertical: VerticalRule::Stretch,
            ..LayoutRules::default()
        },
        children,
    }
}

/// 固定设计像素矩形的叶子。
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
