//! 自 `engine/ra-layout/src/reference/from_template.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-layout/src/reference/from_template.rs :: tests
use ra_layout::{
    RightPanelChrome,
    reference::from_template::*,
    shell::{RectPx, rect_px_from_snapshot},
};

#[test]
fn skirmish_lobby_has_lower_strip_and_bottom_status() {
    let snap = solve_skirmish_lobby();
    assert_eq!(rect_px_from_snapshot(&snap, "lower_strip"), RectPx::new(0, 568, 632, 32));
    assert_eq!(rect_px_from_snapshot(&snap, "status_help"), RectPx::new(15, 579, 455, 20));
    assert_eq!(rect_px_from_snapshot(&snap, "player_name").h, 24);
    assert_eq!(rect_px_from_snapshot(&snap, "flag_0").h, 24);
    assert_eq!(rect_px_from_snapshot(&snap, "side_face_0").h, 24);
    assert_eq!(rect_px_from_snapshot(&snap, "color_face_0").h, 24);
    assert_eq!(rect_px_from_snapshot(&snap, "ai_face_0").h, 24);
}

#[test]
fn skirmish_lobby_form_is_centered_in_content_area() {
    let snap = solve_skirmish_lobby();
    let chrome = RightPanelChrome::shell_defaults();
    let panel_x = chrome.panel_x() as i32;
    let name = rect_px_from_snapshot(&snap, "player_name");
    let color = rect_px_from_snapshot(&snap, "color_face_0");
    let check = rect_px_from_snapshot(&snap, "checkbox_4");
    let left = name.x;
    let right = color.x + color.w;
    let right2 = check.x + check.w;
    let right = right.max(right2);
    let mid = (left + right) / 2;
    let content_mid = panel_x / 2;
    assert!((mid - content_mid).abs() <= 4, "form mid {mid} should near content mid {content_mid} (left={left} right={right})");
    // 不再贴顶：首行应明显低于旧 DLU y≈18。
    assert!(name.y >= 40, "player_name.y={} should leave top margin", name.y);
}

#[test]
fn choose_map_has_lower_strip_and_bottom_status() {
    let snap = solve_choose_map();
    assert_eq!(rect_px_from_snapshot(&snap, "lower_strip"), RectPx::new(0, 568, 632, 32));
    assert_eq!(rect_px_from_snapshot(&snap, "status_help"), RectPx::new(15, 579, 455, 20));
}

#[test]
fn choose_map_lists_are_centered_in_content_area() {
    let snap = solve_choose_map();
    let chrome = RightPanelChrome::shell_defaults();
    let panel_x = chrome.panel_x() as i32;
    let game_type = rect_px_from_snapshot(&snap, "game_type_list");
    let map_list = rect_px_from_snapshot(&snap, "map_list");
    let left = game_type.x;
    let right = map_list.x + map_list.w;
    let mid = (left + right) / 2;
    let content_mid = panel_x / 2;
    assert!((mid - content_mid).abs() <= 4, "lists mid {mid} should near content mid {content_mid}");
    assert!(game_type.y >= 40, "game_type_list.y={} should leave top margin", game_type.y);
    assert_eq!(map_list.x - (game_type.x + game_type.w), 27);
}
