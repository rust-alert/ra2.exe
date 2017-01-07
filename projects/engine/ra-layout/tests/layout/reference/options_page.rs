//! `0xD5` 选项页：`solve_options_page` → snapshot / hit。

use ra_layout::{OPTIONS_CONTENT_IDS, Point2, Rect, solve_options_page};

#[test]
fn dialog_0xd5_rail_matches_shell_tile_snap() {
    let snap = solve_options_page();
    assert_eq!(snap.get("keyboard").map(|e| e.layout.rect), Some(Rect::from_xywh(644.0, 199.0, 156.0, 42.0)));
    assert_eq!(snap.get("network").map(|e| e.layout.rect), Some(Rect::from_xywh(644.0, 241.0, 156.0, 42.0)));
    assert_eq!(snap.get("main_menu").map(|e| e.layout.rect), Some(Rect::from_xywh(644.0, 535.0, 156.0, 42.0)));
    assert_eq!(snap.get("title").map(|e| e.layout.rect), Some(Rect::from_xywh(635.0, 2.0, 162.0, 16.0)));
    assert_eq!(snap.get("status_help").map(|e| e.layout.rect), Some(Rect::from_xywh(15.0, 579.0, 455.0, 20.0)));
    assert_eq!(snap.hit_test(Point2 { x: 650.0, y: 210.0 }).map(|e| e.id.0.as_str()), Some("keyboard"));
    assert_eq!(snap.hit_test(Point2 { x: 650.0, y: 250.0 }).map(|e| e.id.0.as_str()), Some("network"));
    assert_eq!(snap.hit_test(Point2 { x: 650.0, y: 540.0 }).map(|e| e.id.0.as_str()), Some("main_menu"));
}

#[test]
fn dialog_0xd5_left_form_keeps_dlu_sizes_and_gaps() {
    let snap = solve_options_page();
    for id in OPTIONS_CONTENT_IDS {
        assert!(snap.get(id).is_some(), "missing {id}");
    }
    assert!(snap.get("check_observe").is_some());
    assert!(snap.get("rail_badge").is_some());
    assert!(snap.get("accept").is_none());
    assert!(snap.get("sec_present").is_none());

    let track_detail = snap.get("track_detail").unwrap().layout.rect;
    let resolution = snap.get("resolution").unwrap().layout.rect;
    let track_music = snap.get("track_music").unwrap().layout.rect;
    let track_sound = snap.get("track_sound").unwrap().layout.rect;
    let track_voice = snap.get("track_voice").unwrap().layout.rect;
    let caption_detail = snap.get("caption_detail").unwrap().layout.rect;
    let value_detail = snap.get("value_detail").unwrap().layout.rect;

    // DLU→设计像素固有尺寸（居中前/后宽高不变）。
    assert_eq!((track_detail.width, track_detail.height), (180.0, 21.0));
    assert_eq!((resolution.width, resolution.height), (180.0, 24.0));
    // 85 DLU × 6/4 = 127.5 → 四舍五入 128。
    assert_eq!((track_music.width, track_music.height), (128.0, 21.0));
    assert_eq!(track_detail.y, resolution.y);
    assert_eq!(caption_detail.y, value_detail.y);
    assert!((value_detail.x - (caption_detail.x + caption_detail.width)).abs() <= 1.0);

    // 音量三列：同排、等宽、列间距来自模板 DLU。
    assert_eq!(track_music.y, track_sound.y);
    assert_eq!(track_sound.y, track_voice.y);
    assert_eq!(track_music.width, track_sound.width);
    assert_eq!(track_sound.width, track_voice.width);
    let gap_ms = track_sound.x - (track_music.x + track_music.width);
    let gap_sv = track_voice.x - (track_sound.x + track_sound.width);
    assert!((gap_ms - gap_sv).abs() <= 1.0, "audio col gaps {gap_ms} vs {gap_sv}");

    // 左栏表单在内容区大致居中。
    let mid = (track_music.x + track_voice.x + track_voice.width) * 0.5;
    assert!((mid - 316.0).abs() <= 8.0, "audio mid {mid}");
}

#[test]
fn dialog_0xd5_value_labels_sit_above_tracks() {
    let snap = solve_options_page();
    let pairs = [
        ("value_detail", "track_detail"),
        ("value_difficulty", "track_difficulty"),
        ("value_scroll", "track_scroll"),
        ("caption_music", "track_music"),
    ];
    for (label, track) in pairs {
        let l = snap.get(label).unwrap().layout.rect;
        let t = snap.get(track).unwrap().layout.rect;
        assert!(l.y + l.height <= t.y + 1.0, "{label} should sit above {track}");
    }
}
