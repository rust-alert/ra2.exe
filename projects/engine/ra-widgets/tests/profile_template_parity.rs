//! 壳层 profile 对话框经 `solve_*` 落到 snapshot 金标矩形。

use ra_adaptor::shell_runtime_ui_profile;
use ra_layout::{solve_choose_map, solve_skirmish_lobby, Rect};

#[test]
fn shell_profile_0x6b_resolves_choose_map_buttons() {
    let profile = shell_runtime_ui_profile();
    assert!(profile.dialog(0x6B).is_some(), "0x6B");
    let snap = solve_choose_map();
    let use_map = snap.get("use_map").map(|e| e.layout.rect);
    let cancel = snap.get("cancel").map(|e| e.layout.rect);
    assert_eq!(use_map, Some(Rect::from_xywh(644.0, 199.0, 156.0, 42.0)));
    assert_eq!(cancel, Some(Rect::from_xywh(644.0, 535.0, 156.0, 42.0)));
}

#[test]
fn shell_profile_0x102_resolves_skirmish_buttons() {
    let profile = shell_runtime_ui_profile();
    assert!(profile.dialog(0x102).is_some(), "0x102");
    let snap = solve_skirmish_lobby();
    let start = snap.get("start").map(|e| e.layout.rect);
    let back = snap.get("back").map(|e| e.layout.rect);
    assert_eq!(start, Some(Rect::from_xywh(644.0, 241.0, 156.0, 42.0)));
    assert_eq!(back, Some(Rect::from_xywh(644.0, 535.0, 156.0, 42.0)));
}
