//! 集成测试：选项页草稿命中（几何权威为 `solve_options_page`）。

use ra_layout::{rect_px_from_snapshot, solve_options_page};
use ra_types::{DisplayMode, PresentFeel};
use ra_widgets::options_dialog::*;

#[test]
fn rail_accept_is_top_tile_cell() {
    let snap = solve_options_page();
    let accept = rect_px_from_snapshot(&snap, "accept");
    let cancel = rect_px_from_snapshot(&snap, "cancel");
    let main_menu = rect_px_from_snapshot(&snap, "main_menu");
    let panel_tile = rect_px_from_snapshot(&snap, "panel_tile");
    let panel_bottom = rect_px_from_snapshot(&snap, "panel_bottom");
    assert_eq!(accept.y, panel_tile.y);
    assert!(cancel.y > accept.y);
    assert!(main_menu.y < panel_bottom.y);
}

#[test]
fn track_drag_maps_edges() {
    let snap = solve_options_page();
    let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.4, 0.7, PresentFeel::DEFAULT);
    let track = rect_px_from_snapshot(&snap, "track_music");
    state.on_press(track.x + 6, track.y + 4);
    assert_eq!(state.music, 0);
    state.on_press(track.x + track.w - 2, track.y + 4);
    assert_eq!(state.music, 10);
}

#[test]
fn resolution_row_selects_mode() {
    let snap = solve_options_page();
    let mut state = OptionsDialogState::from_shell(DisplayMode::W640H480, 0.5, 0.5, PresentFeel::DEFAULT);
    state.resolution_open = true;
    let row = resolution_row_rect(&snap, 2);
    state.on_press(row.x + 4, row.y + 4);
    assert_eq!(state.display_mode, DisplayMode::W1024H768);
    assert!(!state.resolution_open);
}

#[test]
fn present_toggle_only() {
    let snap = solve_options_page();
    let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.5, 0.5, PresentFeel::DEFAULT);
    let check = rect_px_from_snapshot(&snap, "check_present");
    assert!(state.present.is_active());
    state.on_press(check.x + 4, check.y + 4);
    assert!(!state.present.is_active());
    state.on_press(check.x + 4, check.y + 4);
    assert!(state.present.is_active());
}

#[test]
fn present_controls_fit_content() {
    let snap = solve_options_page();
    let content = rect_px_from_snapshot(&snap, "content");
    let track_voice = rect_px_from_snapshot(&snap, "track_voice");
    let sec_present = rect_px_from_snapshot(&snap, "sec_present");
    let sec_audio = rect_px_from_snapshot(&snap, "sec_audio");
    let bottom = track_voice.y + track_voice.h;
    assert!(bottom <= content.y + content.h);
    assert!(sec_present.y < sec_audio.y);
}
