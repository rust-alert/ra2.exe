//! `LayoutEngine` 集成冒烟。

use ra_layout::{HorizontalRule, LayoutEngine, LayoutNode, LayoutRules, Point2, Size2, SizeRule, VerticalRule, Viewport};

#[test]
fn nested_relative_and_center() {
    let root = LayoutNode {
        id: ra_layout::LayoutId("page".into()),
        rules: LayoutRules { horizontal: HorizontalRule::Stretch, vertical: VerticalRule::Stretch, ..LayoutRules::default() },
        children: vec![LayoutNode::leaf(
            "panel",
            LayoutRules {
                horizontal: HorizontalRule::Center(0.0),
                vertical: VerticalRule::Center(0.0),
                width: SizeRule::Relative(0.5),
                height: SizeRule::Relative(0.5),
                ..LayoutRules::default()
            },
        )],
    };
    let snap = LayoutEngine.solve(Viewport { size: Size2 { width: 800.0, height: 600.0 }, ..Viewport::default() }, &root);
    let panel = snap.get("panel").expect("panel");
    assert_eq!(panel.layout.rect.x, 200.0);
    assert_eq!(panel.layout.rect.y, 150.0);
    assert_eq!(panel.layout.rect.width, 400.0);
    assert_eq!(panel.layout.rect.height, 300.0);
    assert_eq!(snap.hit_test(Point2 { x: 400.0, y: 300.0 }).map(|e| e.id.0.as_str()), Some("panel"));
}
