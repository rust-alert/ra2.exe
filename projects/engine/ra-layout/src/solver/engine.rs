//! 布局引擎：相对父盒定位 + Row/Column 流式排布。

use crate::{
    geometry::{Rect, Size2},
    policy::{HorizontalRule, LayoutFlow, LayoutRules, SizeRule, VerticalRule},
    snapshot::{HitRegion, HitTestMode, LayoutBox, LayoutElement, LayoutSnapshot},
    spec::LayoutNode,
    viewport::Viewport,
};

/// 布局求解器。
#[derive(Debug, Default)]
pub struct LayoutEngine;

impl LayoutEngine {
    /// 对根节点求解，产出扁平快照（父先于子；同层后写入者命中优先）。
    pub fn solve(&self, viewport: Viewport, root: &LayoutNode) -> LayoutSnapshot {
        let parent = Rect::from_xywh(0.0, 0.0, viewport.size.width, viewport.size.height);
        let mut elements = Vec::new();
        Self::solve_node(root, parent, 0, &mut elements);
        LayoutSnapshot { elements }
    }

    fn solve_node(node: &LayoutNode, parent: Rect, z_base: i32, out: &mut Vec<LayoutElement>) {
        let parent_size = Size2 { width: parent.width, height: parent.height };
        let measured = measure_border(node, parent_size);
        let rect = resolve_rect_measured(&node.rules, parent, measured);
        let content = content_box(rect, &node.rules);
        out.push(LayoutElement {
            id: node.id.clone(),
            layout: LayoutBox { rect, clip: None, baseline: None },
            z_index: z_base,
            hit_region: HitRegion { rect },
            hit_test: if node.rules.hit_test { HitTestMode::Rect } else { HitTestMode::None },
        });

        match node.rules.flow {
            LayoutFlow::None => {
                for (i, child) in node.children.iter().enumerate() {
                    Self::solve_node(child, content, z_base + 1 + i as i32, out);
                }
            }
            LayoutFlow::Column { gap } => {
                Self::solve_column(node, content, gap, z_base, out);
            }
            LayoutFlow::Row { gap } => {
                Self::solve_row(node, content, gap, z_base, out);
            }
        }
    }

    fn solve_column(node: &LayoutNode, content: Rect, gap: f32, z_base: i32, out: &mut Vec<LayoutElement>) {
        let mut cursor_y = content.y;
        let parent_size = Size2 { width: content.width, height: content.height };
        for (i, child) in node.children.iter().enumerate() {
            let measured = measure_border(child, parent_size);
            let margin = child.rules.margin;
            let avail_w = (content.width - margin.left - margin.right).max(0.0);
            let width = match child.rules.horizontal {
                HorizontalRule::Stretch => avail_w,
                _ => measured.width,
            };
            let height = measured.height;
            let x = match child.rules.horizontal {
                HorizontalRule::Start(offset) => content.x + margin.left + offset,
                HorizontalRule::Center(offset) => content.x + margin.left + (avail_w - width) * 0.5 + offset,
                HorizontalRule::End(offset) => content.x + content.width - margin.right - width - offset,
                HorizontalRule::Stretch => content.x + margin.left,
            };
            let y = cursor_y + margin.top;
            let placed = LayoutNode {
                id: child.id.clone(),
                rules: LayoutRules {
                    horizontal: HorizontalRule::Start(0.0),
                    vertical: VerticalRule::Top(0.0),
                    width: SizeRule::Fixed(width),
                    height: SizeRule::Fixed(height),
                    margin: Default::default(),
                    padding: child.rules.padding,
                    aspect_ratio: child.rules.aspect_ratio,
                    flow: child.rules.flow,
                    hit_test: child.rules.hit_test,
                },
                children: child.children.clone(),
            };
            let slot_parent = Rect::from_xywh(x, y, width.max(0.0), height.max(0.0));
            Self::solve_node(&placed, slot_parent, z_base + 1 + i as i32, out);
            cursor_y = y + height + margin.bottom;
            if i + 1 < node.children.len() {
                cursor_y += gap;
            }
        }
    }

    fn solve_row(node: &LayoutNode, content: Rect, gap: f32, z_base: i32, out: &mut Vec<LayoutElement>) {
        let mut cursor_x = content.x;
        let parent_size = Size2 { width: content.width, height: content.height };
        for (i, child) in node.children.iter().enumerate() {
            let measured = measure_border(child, parent_size);
            let margin = child.rules.margin;
            let avail_h = (content.height - margin.top - margin.bottom).max(0.0);
            let height = match child.rules.vertical {
                VerticalRule::Stretch => avail_h,
                _ => measured.height,
            };
            let width = measured.width;
            let y = match child.rules.vertical {
                VerticalRule::Top(offset) => content.y + margin.top + offset,
                VerticalRule::Center(offset) => content.y + margin.top + (avail_h - height) * 0.5 + offset,
                VerticalRule::Bottom(offset) => content.y + content.height - margin.bottom - height - offset,
                VerticalRule::Stretch => content.y + margin.top,
            };
            let x = cursor_x + margin.left;
            let placed = LayoutNode {
                id: child.id.clone(),
                rules: LayoutRules {
                    horizontal: HorizontalRule::Start(0.0),
                    vertical: VerticalRule::Top(0.0),
                    width: SizeRule::Fixed(width),
                    height: SizeRule::Fixed(height),
                    margin: Default::default(),
                    padding: child.rules.padding,
                    aspect_ratio: child.rules.aspect_ratio,
                    flow: child.rules.flow,
                    hit_test: child.rules.hit_test,
                },
                children: child.children.clone(),
            };
            let slot_parent = Rect::from_xywh(x, y, width.max(0.0), height.max(0.0));
            Self::solve_node(&placed, slot_parent, z_base + 1 + i as i32, out);
            cursor_x = x + width + margin.right;
            if i + 1 < node.children.len() {
                cursor_x += gap;
            }
        }
    }
}

pub fn content_box(rect: Rect, rules: &LayoutRules) -> Rect {
    let pad = rules.padding;
    Rect::from_xywh(
        rect.x + pad.left,
        rect.y + pad.top,
        (rect.width - pad.left - pad.right).max(0.0),
        (rect.height - pad.top - pad.bottom).max(0.0),
    )
}

pub fn resolve_size(rule: SizeRule, parent_extent: f32, content_extent: f32) -> f32 {
    match rule {
        SizeRule::Content => content_extent.max(0.0),
        SizeRule::Fixed(v) => v.max(0.0),
        SizeRule::Relative(t) => (parent_extent * t).max(0.0),
        SizeRule::Clamp { min, preferred, max } => preferred.clamp(min, max).max(0.0),
    }
}

/// 测量节点边框盒（不含自身 margin；含 padding 与流式子树）。
pub fn measure_border(node: &LayoutNode, parent_size: Size2) -> Size2 {
    let pad = node.rules.padding;
    let (flow_w, flow_h) = match node.rules.flow {
        LayoutFlow::None => (0.0, 0.0),
        LayoutFlow::Column { gap } => {
            let mut w = 0.0f32;
            let mut h = 0.0f32;
            for (i, child) in node.children.iter().enumerate() {
                let cs = measure_border(child, parent_size);
                let m = child.rules.margin;
                w = w.max(cs.width + m.left + m.right);
                h += cs.height + m.top + m.bottom;
                if i + 1 < node.children.len() {
                    h += gap.max(0.0);
                }
            }
            (w, h)
        }
        LayoutFlow::Row { gap } => {
            let mut w = 0.0f32;
            let mut h = 0.0f32;
            for (i, child) in node.children.iter().enumerate() {
                let cs = measure_border(child, parent_size);
                let m = child.rules.margin;
                h = h.max(cs.height + m.top + m.bottom);
                w += cs.width + m.left + m.right;
                if i + 1 < node.children.len() {
                    w += gap.max(0.0);
                }
            }
            (w, h)
        }
    };
    let content_w = flow_w + pad.left + pad.right;
    let content_h = flow_h + pad.top + pad.bottom;

    let width = match node.rules.horizontal {
        HorizontalRule::Stretch => 0.0,
        _ => resolve_size(node.rules.width, parent_size.width, content_w),
    };
    let height = match node.rules.vertical {
        VerticalRule::Stretch => 0.0,
        _ => resolve_size(node.rules.height, parent_size.height, content_h),
    };

    let mut size = Size2 { width, height };
    if let Some(ratio) = node.rules.aspect_ratio {
        if ratio > 0.0 {
            if size.width > 0.0 {
                size.height = size.width / ratio;
            }
            else if size.height > 0.0 {
                size.width = size.height * ratio;
            }
        }
    }
    size
}

pub fn resolve_rect_measured(rules: &LayoutRules, parent: Rect, measured: Size2) -> Rect {
    let margin = rules.margin;
    let avail_w = (parent.width - margin.left - margin.right).max(0.0);
    let avail_h = (parent.height - margin.top - margin.bottom).max(0.0);

    let mut width = match rules.horizontal {
        HorizontalRule::Stretch => avail_w,
        _ => measured.width,
    };
    let mut height = match rules.vertical {
        VerticalRule::Stretch => avail_h,
        _ => measured.height,
    };

    if let Some(ratio) = rules.aspect_ratio {
        if ratio > 0.0 {
            if width > 0.0 && matches!(rules.vertical, VerticalRule::Stretch) {
                // 锁宽时不强制改已拉伸的高。
            }
            else if width > 0.0 && height <= 0.0 {
                height = width / ratio;
            }
            else if height > 0.0 && width <= 0.0 {
                width = height * ratio;
            }
        }
    }

    let x = match rules.horizontal {
        HorizontalRule::Start(offset) => parent.x + margin.left + offset,
        HorizontalRule::Center(offset) => parent.x + margin.left + (avail_w - width) * 0.5 + offset,
        HorizontalRule::End(offset) => parent.x + parent.width - margin.right - width - offset,
        HorizontalRule::Stretch => parent.x + margin.left,
    };
    let y = match rules.vertical {
        VerticalRule::Top(offset) => parent.y + margin.top + offset,
        VerticalRule::Center(offset) => parent.y + margin.top + (avail_h - height) * 0.5 + offset,
        VerticalRule::Bottom(offset) => parent.y + parent.height - margin.bottom - height - offset,
        VerticalRule::Stretch => parent.y + margin.top,
    };

    Rect::from_xywh(x, y, width, height)
}
