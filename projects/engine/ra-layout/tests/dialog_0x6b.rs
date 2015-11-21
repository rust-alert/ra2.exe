//! `0x6B` 选图页：`solve_choose_map` → snapshot / hit。

use ra_layout::{solve_choose_map, Point2, Rect};

#[test]
fn dialog_0x6b_snapshot_matches_choose_map_golden_rects() {
    let snap = solve_choose_map();

    assert_eq!(
        snap.get("use_map").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 241.0, 156.0, 42.0))
    );
    assert_eq!(
        snap.get("create_random").map(|e| e.layout.rect),
        Some(Rect::from_xywh(644.0, 283.0, 156.0, 42.0))
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
                y: 260.0
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
