//! `0x102` 遭遇战大厅：`solve_skirmish_lobby` → snapshot / hit。

use ra_layout::{solve_skirmish_lobby, Point2, Rect};

#[test]
fn dialog_0x102_snapshot_matches_skirmish_golden_rects() {
    let snap = solve_skirmish_lobby();

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
        snap.get("lower_strip").map(|e| e.layout.rect),
        Some(Rect::from_xywh(0.0, 568.0, 632.0, 32.0))
    );
    assert_eq!(
        snap.get("status_help").map(|e| e.layout.rect),
        Some(Rect::from_xywh(15.0, 579.0, 455.0, 20.0))
    );

    // 左栏表单：相对内容区居中后的稳定几何（固有尺寸不变）。
    let player_name = snap.get("player_name").map(|e| e.layout.rect).expect("player_name");
    assert_eq!(player_name.width, 150.0);
    assert_eq!(player_name.height, 24.0);
    let flag = snap.get("flag_0").map(|e| e.layout.rect).expect("flag_0");
    assert_eq!(flag.height, 24.0);
    let color = snap.get("color_face_0").map(|e| e.layout.rect).expect("color_face_0");
    let check4 = snap.get("checkbox_4").map(|e| e.layout.rect).expect("checkbox_4");
    let left = player_name.x;
    let right = (color.x + color.width).max(check4.x + check4.width);
    let mid = (left + right) * 0.5;
    assert!((mid - 316.0).abs() <= 4.0, "form mid {mid}");
    assert!(player_name.y >= 40.0, "top margin y={}", player_name.y);

    assert_eq!(
        snap.get("checkbox_quick").map(|e| {
            let r = e.layout.rect;
            (r.width, r.height)
        }),
        Some((150.0, 16.0))
    );
    assert_eq!(
        snap.get("track_speed").map(|e| {
            let r = e.layout.rect;
            (r.width, r.height)
        }),
        Some((128.0, 21.0))
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
