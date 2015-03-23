//! `RT_DIALOG` `0x102`：adaptor 模板 → `LayoutEngine` → snapshot / hit。

use ra_adaptor::dialog_template_0x102;
use ra_layout::{
    dialog_layout_tree, LayoutEngine, Point2, Rect, RightPanelChrome, Viewport,
};

#[test]
fn dialog_0x102_snapshot_matches_skirmish_golden_rects() {
    let chrome = RightPanelChrome::shell_defaults();
    let root = dialog_layout_tree("dialog_0x102", &dialog_template_0x102(), chrome);
    let snap = LayoutEngine.solve(
        Viewport {
            size: ra_layout::shell_design_size(chrome),
            ..Viewport::default()
        },
        &root,
    );

    assert_eq!(
        snap.get("start").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 241.0, 156.0, 42.0))
    );
    assert_eq!(
        snap.get("choose_map").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 283.0, 156.0, 42.0))
    );
    assert_eq!(
        snap.get("back").map(|e| e.layout.rect),
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
        snap.get("map_name_plate").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 157.0, 156.0, 84.0))
    );
    assert_eq!(
        snap.get("player_name").map(|e| e.layout.rect),
        Some(Rect::from_xywh(53.0, 18.0, 150.0, 20.0))
    );
    assert_eq!(
        snap.get("checkbox_quick").map(|e| e.layout.rect),
        Some(Rect::from_xywh(53.0, 236.0, 150.0, 16.0))
    );
    assert_eq!(
        snap.get("track_speed").map(|e| e.layout.rect),
        Some(Rect::from_xywh(321.0, 236.0, 128.0, 21.0))
    );
    assert_eq!(
        snap.get("label_speed").map(|e| e.layout.rect),
        Some(Rect::from_xywh(219.0, 236.0, 90.0, 16.0))
    );

    assert_eq!(
        snap
            .hit_test(Point2 {
                x: 650.0,
                y: 250.0
            })
            .map(|e| e.id.0.as_str()),
        Some("start")
    );
}
