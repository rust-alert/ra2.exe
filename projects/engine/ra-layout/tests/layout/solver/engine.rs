//! 自 `engine/ra-layout/src/solver/engine.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/solver/engine.rs :: tests
use ra_layout::{
    HitTestMode, HorizontalRule, LayoutEngine, LayoutId, LayoutNode, LayoutRules, Point2, Rect, Size2, SizeRule, VerticalRule, Viewport,
    column, row, sized_leaf,
};

#[test]
fn solve_fixed_child_relative_to_viewport() {
    let engine = LayoutEngine;
    let root = LayoutNode {
        id: LayoutId("root".into()),
        rules: LayoutRules { horizontal: HorizontalRule::Stretch, vertical: VerticalRule::Stretch, ..LayoutRules::default() },
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
    let vp = Viewport { size: Size2 { width: 800.0, height: 600.0 }, ..Viewport::default() };
    let snap = engine.solve(vp, &root);
    assert_eq!(snap.elements.len(), 2);
    let btn = snap.get("btn").expect("btn");
    assert_eq!(btn.layout.rect, Rect::from_xywh(10.0, 20.0, 100.0, 40.0));
    assert_eq!(btn.hit_region.rect, btn.layout.rect);
    assert!(snap.hit_test(Point2 { x: 15.0, y: 25.0 }).is_some_and(|e| e.id.0 == "btn"));
}

#[test]
fn column_stacks_with_gap_and_content_size() {
    let engine = LayoutEngine;
    let root = LayoutNode {
        id: LayoutId("root".into()),
        rules: LayoutRules { horizontal: HorizontalRule::Stretch, vertical: VerticalRule::Stretch, ..LayoutRules::default() },
        children: vec![column("col", 8.0, vec![sized_leaf("a", 40.0, 10.0), sized_leaf("b", 20.0, 10.0)])],
    };
    let snap = engine.solve(Viewport { size: Size2 { width: 200.0, height: 200.0 }, ..Viewport::default() }, &root);
    let col = snap.get("col").expect("col");
    assert_eq!(col.layout.rect, Rect::from_xywh(0.0, 0.0, 40.0, 28.0));
    assert_eq!(col.hit_test, HitTestMode::None);
    assert_eq!(snap.get("a").map(|e| e.layout.rect), Some(Rect::from_xywh(0.0, 0.0, 40.0, 10.0)));
    assert_eq!(snap.get("b").map(|e| e.layout.rect), Some(Rect::from_xywh(0.0, 18.0, 20.0, 10.0)));
}

#[test]
fn row_centers_in_parent_and_aligns_cross_axis() {
    let engine = LayoutEngine;
    let mut form = row("form", 4.0, vec![sized_leaf("l", 30.0, 10.0), sized_leaf("r", 30.0, 20.0)]);
    form.rules.horizontal = HorizontalRule::Center(0.0);
    form.rules.vertical = VerticalRule::Center(0.0);
    let root = LayoutNode {
        id: LayoutId("root".into()),
        rules: LayoutRules { horizontal: HorizontalRule::Stretch, vertical: VerticalRule::Stretch, ..LayoutRules::default() },
        children: vec![form],
    };
    let snap = engine.solve(Viewport { size: Size2 { width: 200.0, height: 100.0 }, ..Viewport::default() }, &root);
    // form 固有 30+4+30=64 宽、20 高，居中于 200x100 → (68, 40)
    assert_eq!(snap.get("form").map(|e| e.layout.rect), Some(Rect::from_xywh(68.0, 40.0, 64.0, 20.0)));
    assert_eq!(snap.get("l").map(|e| e.layout.rect), Some(Rect::from_xywh(68.0, 40.0, 30.0, 10.0)));
    assert_eq!(snap.get("r").map(|e| e.layout.rect), Some(Rect::from_xywh(102.0, 40.0, 30.0, 20.0)));
}

#[test]
fn column_cross_align_center() {
    let engine = LayoutEngine;
    let mut a = sized_leaf("a", 10.0, 10.0);
    a.rules.horizontal = HorizontalRule::Center(0.0);
    let col = column("col", 0.0, vec![a, sized_leaf("b", 40.0, 10.0)]);
    let root = LayoutNode {
        id: LayoutId("root".into()),
        rules: LayoutRules { horizontal: HorizontalRule::Stretch, vertical: VerticalRule::Stretch, ..LayoutRules::default() },
        children: vec![col],
    };
    let snap = engine.solve(Viewport { size: Size2 { width: 100.0, height: 100.0 }, ..Viewport::default() }, &root);
    // col 宽 40；a 居中 → x=15
    assert_eq!(snap.get("a").map(|e| e.layout.rect), Some(Rect::from_xywh(15.0, 0.0, 10.0, 10.0)));
}

#[test]
fn flow_container_skips_hit_test() {
    let engine = LayoutEngine;
    let root = LayoutNode {
        id: LayoutId("root".into()),
        rules: LayoutRules { horizontal: HorizontalRule::Stretch, vertical: VerticalRule::Stretch, ..LayoutRules::default() },
        children: vec![column("col", 0.0, vec![sized_leaf("a", 20.0, 20.0)])],
    };
    let snap = engine.solve(Viewport { size: Size2 { width: 100.0, height: 100.0 }, ..Viewport::default() }, &root);
    assert_eq!(snap.hit_test(Point2 { x: 5.0, y: 5.0 }).map(|e| e.id.0.as_str()), Some("a"));
    assert_ne!(snap.hit_test(Point2 { x: 5.0, y: 5.0 }).map(|e| e.id.0.as_str()), Some("col"));
}
