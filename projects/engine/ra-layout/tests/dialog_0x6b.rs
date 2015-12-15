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

    // 左栏双列表：内容区居中后固有尺寸与相对间距不变。
    let game_type_list = snap.get("game_type_list").map(|e| e.layout.rect).expect("game_type_list");
    let map_list = snap.get("map_list").map(|e| e.layout.rect).expect("map_list");
    assert_eq!((game_type_list.width, game_type_list.height), (195.0, 260.0));
    assert_eq!((map_list.width, map_list.height), (195.0, 260.0));
    assert_eq!(game_type_list.y, map_list.y);
    assert_eq!(map_list.x - (game_type_list.x + game_type_list.width), 27.0);
    let mid = (game_type_list.x + map_list.x + map_list.width) * 0.5;
    assert!((mid - 316.0).abs() <= 4.0, "lists mid {mid}");
    assert!(game_type_list.y >= 40.0, "top margin y={}", game_type_list.y);

    assert_eq!(
        snap.get("lower_strip").map(|e| e.layout.rect),
        Some(Rect::from_xywh(0.0, 568.0, 632.0, 32.0))
    );
    assert_eq!(
        snap.get("status_help").map(|e| e.layout.rect),
        Some(Rect::from_xywh(15.0, 579.0, 455.0, 20.0))
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
