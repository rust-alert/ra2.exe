//! 自 `engine/ra-layout/src/reference/options_page.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/options_page.rs :: tests
use ra_layout::{
    LayoutEngine, RightPanelChrome, Viewport,
    reference::{from_template::shell_design_size, options_page::*},
    shell::rect_px_from_snapshot,
};

#[test]
fn options_content_matches_800x600_golden() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &options_content_layout_tree(chrome));
    let expect = [
        ("content", 16, 16, 608, 548),
        ("sec_display", 32, 28, 576, 18),
        ("track_detail", 32, 68, 280, 22),
        ("resolution", 328, 68, 280, 28),
        ("sec_game", 32, 112, 576, 18),
        ("track_difficulty", 32, 152, 536, 22),
        ("sec_ui", 32, 190, 576, 18),
        ("check_tooltips", 32, 230, 220, 22),
        ("check_scanlines", 32, 254, 220, 22),
        ("check_damage", 32, 278, 220, 22),
        ("track_scroll", 328, 230, 280, 22),
        ("sec_present", 32, 316, 576, 18),
        ("check_present", 32, 356, 280, 22),
        ("sec_audio", 32, 394, 576, 18),
        ("track_music", 32, 434, 536, 22),
        ("track_sound", 32, 474, 536, 22),
        ("track_voice", 32, 514, 536, 22),
    ];
    for (id, x, y, w, h) in expect {
        let got = snap.get(id).expect(id).layout.rect;
        assert_eq!(got.x as i32, x, "{id} x");
        assert_eq!(got.y as i32, y, "{id} y");
        assert_eq!(got.width as i32, w, "{id} w");
        assert_eq!(got.height as i32, h, "{id} h");
    }
    assert_eq!(OPTIONS_CONTENT_IDS.len(), expect.len());
    // 内容板不得盖住底条。
    let content = rect_px_from_snapshot(&snap, "content");
    assert!(content.y + content.h <= 568, "content overlaps lower_strip");
}

#[test]
fn options_page_tree_includes_rail_and_content() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &options_page_layout_tree(chrome));
    assert!(snap.get("panel_top").is_some());
    assert!(snap.get("lower_strip").is_some());
    assert!(snap.get("accept").is_some());
    assert!(snap.get("main_menu").is_some());
    assert!(snap.get("content").is_some());
    assert!(snap.get("track_voice").is_some());
    let content = options_content_layout_tree(chrome);
    let content_snap = LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &content);
    assert_eq!(snap.get("track_detail").map(|e| e.layout.rect), content_snap.get("track_detail").map(|e| e.layout.rect));
}

#[test]
fn options_track_labels_have_clearance_above_tracks() {
    let chrome = RightPanelChrome::shell_defaults();
    let snap = LayoutEngine.solve(Viewport { size: shell_design_size(chrome), ..Viewport::default() }, &options_content_layout_tree(chrome));
    let pairs = [("sec_display", "track_detail"), ("sec_game", "track_difficulty"), ("sec_audio", "track_music")];
    for (sec, track) in pairs {
        let s = snap.get(sec).expect(sec).layout.rect;
        let t = snap.get(track).expect(track).layout.rect;
        // 分区底到滑条顶：分隔线 + 一行标签（`SEC_TO_TRACK - SEC_H` ≈ 22）。
        assert!(t.y >= s.y + s.height + 20.0, "{sec}->{track}: gap too small ({} vs {})", t.y, s.y + s.height);
    }
    let music = snap.get("track_music").unwrap().layout.rect;
    let sound = snap.get("track_sound").unwrap().layout.rect;
    // `TRACK_STACK - TRACK_H` = 18，刚好塞下上一行标签尾与下一行标签。
    assert!(sound.y >= music.y + music.height + 16.0, "audio tracks stacked too tight");
}
