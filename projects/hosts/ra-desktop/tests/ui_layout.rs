//! 集成测试：原 `src/ui_layout.rs` 内联测试迁出。

use ra_desktop::ui_layout::*;
#[test]
fn main_menu_panel_sits_on_right_edge() {
    let layout = main_menu_layout(1024, 768);
    assert_eq!(layout.canvas.w, 800);
    assert_eq!(layout.panel_top.x + layout.panel_top.w, 800);
    assert!(layout.panel_tile_count > 0);
    assert_eq!(layout.buttons[0].w, BUTTON_CELL_W);
    assert_eq!(layout.buttons[0].y, RIGHT_PANEL_TOP_H);
    assert_eq!(layout.buttons[3].y, RIGHT_PANEL_TOP_H + 3 * BUTTON_CELL_H);
    assert_eq!(MAIN_MENU_BUTTON_IDS.len(), layout.buttons.len());
}

#[test]
fn skirmish_lobby_reuses_right_panel() {
    let layout = skirmish_lobby_layout(1024, 768);
    assert_eq!(SKIRMISH_LOBBY_BUTTON_IDS.len(), layout.buttons.len());
    assert_eq!(layout.buttons[0].y, RIGHT_PANEL_TOP_H);
    let row0 = skirmish_map_row_rect(&layout, 0);
    assert!(row0.w > 0);
    assert!(row0.y >= layout.movie.y);
}

#[test]
fn window_center_maps_near_shell_center_when_fitted() {
    let (x, y) = window_to_shell_px(512.0, 384.0, 1024.0, 768.0);
    assert!((x - 400).abs() <= 2);
    assert!((y - 300).abs() <= 2);
}
