//! 集成测试：选项页草稿命中（几何权威为 `solve_options_page` / `0xD5`）。

use ra_layout::{rect_px_from_snapshot, solve_options_page};
use ra_types::{DisplayMode, PresentFeel};
use ra_widgets::options_dialog::*;

#[test]
fn rail_keyboard_is_top_tile_cell() {
    let snap = solve_options_page();
    let keyboard = rect_px_from_snapshot(&snap, "keyboard");
    let network = rect_px_from_snapshot(&snap, "network");
    let main_menu = rect_px_from_snapshot(&snap, "main_menu");
    let panel_tile = rect_px_from_snapshot(&snap, "panel_tile");
    let panel_bottom = rect_px_from_snapshot(&snap, "panel_bottom");
    assert_eq!(keyboard.y, panel_tile.y);
    assert!(network.y > keyboard.y);
    // `main_menu` 贴在 `panel_bottom` 上沿（`bottom_cover_button`）。
    assert_eq!(main_menu.y, panel_bottom.y);
}

#[test]
fn track_drag_maps_edges() {
    let snap = solve_options_page();
    let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.4, 0.7, PresentFeel::DEFAULT);
    let track = rect_px_from_snapshot(&snap, "track_music");
    state.on_press(track.x + 6, track.y + 4);
    assert_eq!(state.music, 0);
    state.on_press(track.x + track.w - 30, track.y + 4);
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
fn audio_tracks_are_side_by_side() {
    let snap = solve_options_page();
    let music = rect_px_from_snapshot(&snap, "track_music");
    let sound = rect_px_from_snapshot(&snap, "track_sound");
    let voice = rect_px_from_snapshot(&snap, "track_voice");
    assert_eq!(music.y, sound.y);
    assert_eq!(sound.y, voice.y);
    assert!(sound.x > music.x + music.w);
    assert!(voice.x > sound.x + sound.w);
    assert!(snap.get("sec_present").is_none());
}
