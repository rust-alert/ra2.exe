//! `RT_DIALOG` `0x6B`：adaptor 模板 → `LayoutEngine` → snapshot / hit。

use ra_adaptor::dialog_template_0x6b;
use ra_layout::{
    dialog_layout_tree, LayoutEngine, Point2, Rect, RightPanelChrome, Viewport,
};

#[test]
fn dialog_0x6b_snapshot_matches_choose_map_golden_rects() {
    let chrome = RightPanelChrome::shell_defaults();
    let root = dialog_layout_tree("dialog_0x6b", &dialog_template_0x6b(), chrome);
    let snap = LayoutEngine.solve(
        Viewport {
            size: ra_layout::shell_design_size(chrome),
            ..Viewport::default()
        },
        &root,
    );

    assert_eq!(
        snap.get("use_map").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 199.0, 156.0, 42.0))
    );
    assert_eq!(
        snap.get("create_random").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 241.0, 156.0, 42.0))
    );
    assert_eq!(
        snap.get("cancel").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 535.0, 156.0, 42.0))
    );
    assert_eq!(
        snap.get("map_preview").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 37.0, 144.0, 112.0))
    );
    assert_eq!(
        snap.get("title").map(|e| e.layout.rect),
        Some(Rect::from_xywh(635.0, 2.0, 162.0, 16.0))
    );
    assert_eq!(
        snap.get("game_type_list").map(|e| e.layout.rect),
        Some(Rect::from_xywh(30.0, 127.0, 195.0, 260.0))
    );
    assert_eq!(
        snap.get("map_list").map(|e| e.layout.rect),
        Some(Rect::from_xywh(252.0, 127.0, 195.0, 260.0))
    );

    assert_eq!(
        snap
            .hit_test(Point2 {
                x: 650.0,
                y: 210.0
            })
            .map(|e| e.id.0.as_str()),
        Some("use_map")
    );
    assert_eq!(
        snap
            .hit_test(Point2 {
                x: 650.0,
                y: 540.0
            })
            .map(|e| e.id.0.as_str()),
        Some("cancel")
    );
}
