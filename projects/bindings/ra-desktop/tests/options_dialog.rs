//! 集成测试：原 `src/options_dialog.rs` 内联测试迁出。

use ra_desktop::{options_dialog::*, ui_layout::main_menu_layout};
use ra_types::{DisplayMode, PresentFeel};

#[test]
fn rail_accept_is_top_tile_cell() {
    let layout = OptionsDialogLayout::new();
    let shell = main_menu_layout(0, 0);
    assert_eq!(layout.rail[0].y, shell.panel_tile.y);
    assert!(layout.rail[2].y < shell.panel_bottom.y);
}

#[test]
fn track_drag_maps_edges() {
    let layout = OptionsDialogLayout::new();
    let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.4, 0.7, PresentFeel::DEFAULT);
    let track = layout.track_music;
    state.on_press(&layout, track.x + 6, track.y + 4);
    assert_eq!(state.music, 0);
    state.on_press(&layout, track.x + track.w - 2, track.y + 4);
    assert_eq!(state.music, 10);
}

#[test]
fn resolution_row_selects_mode() {
    let layout = OptionsDialogLayout::new();
    let mut state = OptionsDialogState::from_shell(DisplayMode::W640H480, 0.5, 0.5, PresentFeel::DEFAULT);
    state.resolution_open = true;
    let row = layout.resolution_row(2);
    state.on_press(&layout, row.x + 4, row.y + 4);
    assert_eq!(state.display_mode, DisplayMode::W1024H768);
    assert!(!state.resolution_open);
}

#[test]
fn present_toggle_only() {
    let layout = OptionsDialogLayout::new();
    let mut state = OptionsDialogState::from_shell(DisplayMode::W800H600, 0.5, 0.5, PresentFeel::DEFAULT);
    assert!(state.present.is_active());
    state.on_press(&layout, layout.check_present.x + 4, layout.check_present.y + 4);
    assert!(!state.present.is_active());
    state.on_press(&layout, layout.check_present.x + 4, layout.check_present.y + 4);
    assert!(state.present.is_active());
}

#[test]
fn present_controls_fit_content() {
    let layout = OptionsDialogLayout::new();
    let bottom = layout.track_voice.y + layout.track_voice.h;
    assert!(bottom <= layout.content.y + layout.content.h);
    assert!(layout.sec_present.y < layout.sec_audio.y);
}
