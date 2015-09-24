//! 选图列表滚动窗口纯函数。

use ra_layout::{clamp_map_list_scroll, scroll_map_list_to_reveal};

#[test]
fn clamp_scroll_when_all_fit() {
    assert_eq!(clamp_map_list_scroll(5, 10, 20), 0);
    assert_eq!(clamp_map_list_scroll(0, 3, 3), 0);
}

#[test]
fn clamp_scroll_caps_at_last_window() {
    assert_eq!(clamp_map_list_scroll(0, 36, 10), 0);
    assert_eq!(clamp_map_list_scroll(100, 36, 10), 26);
    assert_eq!(clamp_map_list_scroll(20, 36, 10), 20);
}

#[test]
fn reveal_moves_window_to_include_index() {
    assert_eq!(scroll_map_list_to_reveal(0, 0, 36, 10), 0);
    assert_eq!(scroll_map_list_to_reveal(0, 9, 36, 10), 0);
    assert_eq!(scroll_map_list_to_reveal(0, 10, 36, 10), 1);
    assert_eq!(scroll_map_list_to_reveal(5, 3, 36, 10), 3);
    assert_eq!(scroll_map_list_to_reveal(5, 35, 36, 10), 26);
}
