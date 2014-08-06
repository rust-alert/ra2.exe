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
    assert_eq!(MAIN_MENU_BUTTON_IDS.len(), layout.buttons.len());
}

#[test]
fn main_menu_bottom_cover_is_remainder_not_fixed_65() {
    let layout = main_menu_layout(800, 600);
    // 600 - 199 = 401；401/42 = 9 格；底盖 y=199+378=577，高=23。
    assert_eq!(layout.panel_tile_count, 9);
    assert_eq!(layout.panel_bottom, RectPx::new(632, 577, 168, 23));
    assert_eq!(layout.movie, RectPx::new(0, 0, 632, 570));
    assert_eq!(layout.lower_strip, RectPx::new(0, 568, 632, 32));
}

#[test]
fn main_menu_title_and_tooltip_rects() {
    let layout = main_menu_layout(800, 600);
    assert_eq!(layout.title, RectPx::new(635, 9, 163, 18));
    assert_eq!(layout.tooltip, RectPx::new(10, 579, 455, 20));
}

#[test]
fn main_menu_exit_sits_on_bottom_cover() {
    let layout = main_menu_layout(800, 600);
    let expected_y = [199, 241, 283, 325, 367];
    for (i, y) in expected_y.iter().enumerate() {
        assert_eq!(layout.buttons[i], RectPx::new(644, *y, 156, 42));
    }
    // Exit：底盖上沿一行 → y = 577 - 42 = 535。
    assert_eq!(layout.buttons[5], RectPx::new(644, 535, 156, 42));
}

#[test]
fn skirmish_lobby_reuses_right_panel() {
    let layout = skirmish_lobby_layout(1024, 768);
    assert_eq!(SKIRMISH_LOBBY_BUTTON_IDS.len(), 4);
    assert_eq!(layout.shell.buttons[0].y, RIGHT_PANEL_TOP_H);
    assert_eq!(layout.shell.buttons[3].y, layout.shell.panel_bottom.y - BUTTON_CELL_H);
    let row0 = skirmish_map_row_rect(&layout, 0);
    assert!(row0.w > 0);
    assert!(row0.y >= layout.shell.movie.y);
}

#[test]
fn exit_confirm_centers_pudlgbgn_panel() {
    let dlg = exit_confirm_layout(800, 600);
    assert_eq!(dlg.dialog, RectPx::new(175, 138, EXIT_CONFIRM_DIALOG_W, EXIT_CONFIRM_DIALOG_H));
    // 正文与右侧纵向确定/取消（DLU→像素）。
    assert_eq!(dlg.prompt, RectPx::new(235, 203, 330, 81));
    assert_eq!(dlg.buttons[0], RectPx::new(486, 357, 125, 24));
    assert_eq!(dlg.buttons[1], RectPx::new(486, 422, 125, 24));
}

