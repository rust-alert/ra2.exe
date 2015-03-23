//! 壳层 profile 对话框经 layout 策略解析到金标矩形。

use ra_adaptor::shell_runtime_ui_profile;
use ra_layout::{resolve_dialog_template, Rect, RightPanelChrome};

#[test]
fn shell_profile_0x6b_resolves_choose_map_buttons() {
    let chrome = RightPanelChrome::shell_defaults();
    let profile = shell_runtime_ui_profile();
    let template = profile.dialog(0x6B).expect("0x6B");
    let rects = resolve_dialog_template(template, chrome);
    let use_map = rects.iter().find(|(id, _)| id == "use_map").map(|(_, r)| *r);
    let cancel = rects.iter().find(|(id, _)| id == "cancel").map(|(_, r)| *r);
    assert_eq!(use_map, Some(Rect::from_xywh(644.0, 199.0, 156.0, 42.0)));
    assert_eq!(cancel, Some(Rect::from_xywh(644.0, 535.0, 156.0, 42.0)));
}

#[test]
fn shell_profile_0x102_resolves_skirmish_buttons() {
    let chrome = RightPanelChrome::shell_defaults();
    let profile = shell_runtime_ui_profile();
    let template = profile.dialog(0x102).expect("0x102");
    let rects = resolve_dialog_template(template, chrome);
    let start = rects.iter().find(|(id, _)| id == "start").map(|(_, r)| *r);
    let back = rects.iter().find(|(id, _)| id == "back").map(|(_, r)| *r);
    assert_eq!(start, Some(Rect::from_xywh(644.0, 241.0, 156.0, 42.0)));
    assert_eq!(back, Some(Rect::from_xywh(644.0, 535.0, 156.0, 42.0)));
}
