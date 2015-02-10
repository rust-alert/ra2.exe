//! 布局引擎（最小可用：相对父盒定位与尺寸，尚无完整约束求解）。

use crate::{
    geometry::Rect,
    policy::{HorizontalRule, LayoutRules, SizeRule, VerticalRule},
    snapshot::{HitRegion, HitTestMode, LayoutBox, LayoutElement, LayoutSnapshot},
    spec::LayoutNode,
    viewport::Viewport,
};

/// 布局求解器。
#[derive(Debug, Default)]
pub struct LayoutEngine;

impl LayoutEngine {
    /// 对根节点求解，产出扁平快照（子先于父的深度优先序，便于后绘覆盖）。
    pub fn solve(&self, viewport: Viewport, root: &LayoutNode) -> LayoutSnapshot {
        let parent = Rect::from_xywh(0.0, 0.0, viewport.size.width, viewport.size.height);
        let mut elements = Vec::new();
        Self::solve_node(root, parent, 0, &mut elements);
        LayoutSnapshot { elements }
    }

    fn solve_node(node: &LayoutNode, parent: Rect, z_base: i32, out: &mut Vec<LayoutElement>) {
        let rect = resolve_rect(&node.rules, parent);
        let content = content_box(rect, &node.rules);
        // 先父后子：绘制与命中同序，末尾元素在上层。
        out.push(LayoutElement {
            id: node.id.clone(),
            layout: LayoutBox {
                rect,
                clip: None,
                baseline: None,
            },
            z_index: z_base,
            hit_region: HitRegion { rect },
            hit_test: HitTestMode::Rect,
        });
        for (i, child) in node.children.iter().enumerate() {
            Self::solve_node(child, content, z_base + 1 + i as i32, out);
        }
    }
}

fn content_box(rect: Rect, rules: &LayoutRules) -> Rect {
    let pad = rules.padding;
    Rect::from_xywh(
        rect.x + pad.left,
        rect.y + pad.top,
        (rect.width - pad.left - pad.right).max(0.0),
        (rect.height - pad.top - pad.bottom).max(0.0),
    )
}

fn resolve_size(rule: SizeRule, parent_extent: f32) -> f32 {
    match rule {
        SizeRule::Content => 0.0,
        SizeRule::Fixed(v) => v.max(0.0),
        SizeRule::Relative(t) => (parent_extent * t).max(0.0),
        SizeRule::Clamp {
            min,
            preferred,
            max,
        } => preferred.clamp(min, max).max(0.0),
    }
}

fn resolve_rect(rules: &LayoutRules, parent: Rect) -> Rect {
    let margin = rules.margin;
    let avail_w = (parent.width - margin.left - margin.right).max(0.0);
    let avail_h = (parent.height - margin.top - margin.bottom).max(0.0);

    let mut width = match rules.horizontal {
        HorizontalRule::Stretch => avail_w,
        _ => resolve_size(rules.width, parent.width),
    };
    let mut height = match rules.vertical {
        VerticalRule::Stretch => avail_h,
        _ => resolve_size(rules.height, parent.height),
    };

    if let Some(ratio) = rules.aspect_ratio {
        if ratio > 0.0 {
            // 以已解析的非零边为准；两边皆有时优先锁宽。
            if width > 0.0 {
                height = width / ratio;
            } else if height > 0.0 {
                width = height * ratio;
            }
        }
    }

    let x = match rules.horizontal {
        HorizontalRule::Start(offset) => parent.x + margin.left + offset,
        HorizontalRule::Center(offset) => {
            parent.x + margin.left + (avail_w - width) * 0.5 + offset
        }
        HorizontalRule::End(offset) => parent.x + parent.width - margin.right - width - offset,
        HorizontalRule::Stretch => parent.x + margin.left,
    };
    let y = match rules.vertical {
        VerticalRule::Top(offset) => parent.y + margin.top + offset,
        VerticalRule::Center(offset) => {
            parent.y + margin.top + (avail_h - height) * 0.5 + offset
        }
        VerticalRule::Bottom(offset) => parent.y + parent.height - margin.bottom - height - offset,
        VerticalRule::Stretch => parent.y + margin.top,
    };

    Rect::from_xywh(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::{Point2, Size2}, spec::LayoutId};

    #[test]
    fn solve_fixed_child_relative_to_viewport() {
        let engine = LayoutEngine;
        let root = LayoutNode {
            id: LayoutId("root".into()),
            rules: LayoutRules {
                horizontal: HorizontalRule::Stretch,
                vertical: VerticalRule::Stretch,
                ..LayoutRules::default()
            },
            children: vec![LayoutNode::leaf(
                "btn",
                LayoutRules {
                    horizontal: HorizontalRule::Start(10.0),
                    vertical: VerticalRule::Top(20.0),
                    width: SizeRule::Fixed(100.0),
                    height: SizeRule::Fixed(40.0),
                    ..LayoutRules::default()
                },
            )],
        };
        let vp = Viewport {
            size: Size2 {
                width: 800.0,
                height: 600.0,
            },
            ..Viewport::default()
        };
        let snap = engine.solve(vp, &root);
        assert_eq!(snap.elements.len(), 2);
        let btn = snap.get("btn").expect("btn");
        assert_eq!(btn.layout.rect, Rect::from_xywh(10.0, 20.0, 100.0, 40.0));
        assert_eq!(btn.hit_region.rect, btn.layout.rect);
        assert!(snap
            .hit_test(Point2 { x: 15.0, y: 25.0 })
            .is_some_and(|e| e.id.0 == "btn"));
    }
}
