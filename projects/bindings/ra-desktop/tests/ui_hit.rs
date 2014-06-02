//! 集成测试：原 `src/ui_hit.rs` 内联测试迁出。

use ra_desktop::{
    boot::BootMapCandidate,
    menu_action::MenuAction,
    screen::OriginalScreen,
    ui_hit::*,
    ui_layout::{skirmish_lobby_layout, skirmish_map_row_rect},
};
use ra_map::Theater;

#[test]
fn main_menu_hit_single_player() {
    // 右侧首钮格中心：壳层约 (722, 220) → 1024×768 fit 后约 (924, 282)
    let action = hit_action(OriginalScreen::MainMenu, &[], None, (924.0, 282.0), 1024.0, 768.0, false);
    assert_eq!(action, Some(MenuAction::OpenSinglePlayer));
}

#[test]
fn load_screen_disables_retry_while_loading() {
    let loading = hits_for(OriginalScreen::LoadScreen, &[], false);
    let retry = loading.iter().find(|h| h.entry_id == "retry").expect("retry");
    assert!(!retry.enabled);
    let failed = hits_for(OriginalScreen::LoadScreen, &[], true);
    let retry = failed.iter().find(|h| h.entry_id == "retry").expect("retry");
    assert!(retry.enabled);
}

#[test]
fn disabled_network_not_hit() {
    // 网络钮格中心约壳层 (722, 262) → 窗口约 (924, 335)，禁用。
    let action = hit_action(OriginalScreen::MainMenu, &[], None, (924.0, 335.0), 1024.0, 768.0, false);
    assert_eq!(action, None);
}

#[test]
fn lobby_map_row_is_selectable() {
    let maps = vec![BootMapCandidate { file_name: "mp03t4.map".into(), width: 50, height: 50, theater: Theater::Temperate }];
    let layout = skirmish_lobby_layout(0, 0);
    let row = skirmish_map_row_rect(&layout, 0);
    let cx = row.x + row.w / 2;
    let cy = row.y + row.h / 2;
    let cam = ra_desktop::ui_layout::shell_fit_camera(1024, 768);
    let sx = (cx as f32 - cam.center_x) * cam.zoom + 1024.0 * 0.5;
    let sy = (cy as f32 - cam.center_y) * cam.zoom + 768.0 * 0.5;
    let action = hit_action(OriginalScreen::SkirmishLobby, &maps, Some("mp03t4.map"), (sx as f64, sy as f64), 1024.0, 768.0, false);
    assert_eq!(action, Some(MenuAction::SelectMap(0)));
}

#[test]
fn hover_index_tracks_enabled_button() {
    assert_eq!(hover_index(OriginalScreen::MainMenu, &[], None, (924.0, 282.0), 1024.0, 768.0, false,), Some(0));
    assert_eq!(hover_index(OriginalScreen::MainMenu, &[], None, (924.0, 335.0), 1024.0, 768.0, false,), None);
}

#[test]
fn physical_cursor_with_logical_window_misses_on_hidpi() {
    // 回归：逻辑窗 1024×768、缩放 1.5 时，若误用物理光标 (1386,423) 会打飞命中。
    // 生产路径必须先把 CursorMoved 转成逻辑像素 (924,282)。
    let logical = hit_action(OriginalScreen::MainMenu, &[], None, (924.0, 282.0), 1024.0, 768.0, false);
    let physical_mixed = hit_action(OriginalScreen::MainMenu, &[], None, (1386.0, 423.0), 1024.0, 768.0, false);
    assert_eq!(logical, Some(MenuAction::OpenSinglePlayer));
    assert_eq!(physical_mixed, None);
}
