//! 自 `engine/ra-widgets/src/screens/battle_order_icons.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-widgets/src/screens/battle_order_icons.rs :: hotspot_tests
use ra_widgets::screens::battle_order_icons::*;

#[test]
fn scroll_hotspot_center_top_on_even_canvas() {
    let (x, y) = MouseCursorHotspot::CenterTop.to_px(64, 48);
    assert_eq!((x, y), (32, 0));
}

#[test]
fn scroll_ring_frame_offsets_match_table() {
    assert_eq!(MOUSE_SCROLL_START + 7, 9);
    assert_eq!(MOUSE_SCROLL_BLOCKED_START + 7, 17);
}

#[test]
fn core_gameplay_frame_table() {
    assert_eq!(MOUSE_SELECT_START + MOUSE_SELECT_LEN - 1, 30);
    assert_eq!(MOUSE_MOVE_START + MOUSE_MOVE_LEN - 1, 40);
    assert_eq!(MOUSE_NO_MOVE_START, 41);
    assert_eq!(MOUSE_ATTACK_START + MOUSE_ATTACK_LEN - 1, 57);
    assert_eq!(MOUSE_NO_DEPLOY_START, 119);
}
