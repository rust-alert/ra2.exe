//! 集成测试：原 `src/ui_hit.rs` 内联测试迁出。

use ra_components::{menu_action::MenuAction, original_screen::OriginalScreen, ui_hit::*};
use ra_layout::ui_layout::skirmish_lobby_layout;
use ra_map::{BootMapCandidate, Theater};

#[test]
fn main_menu_hit_single_player() {
    // 右侧首钮格中心：壳层约 (722, 220) → 1024×768 fit 后约 (924, 282)
    let action = hit_action(OriginalScreen::MainMenu, &[], None, (924.0, 282.0), 1024.0, 768.0, false);
    assert_eq!(action, Some(MenuAction::OpenSinglePlayer));
}

#[test]
fn main_menu_hit_options_and_exit_cells() {
    let cam = ra_layout::ui_layout::shell_fit_camera(1024, 768);
    let to_win = |sx: i32, sy: i32| {
        let x = (sx as f32 - cam.center_x) * cam.zoom + 1024.0 * 0.5;
        let y = (sy as f32 - cam.center_y) * cam.zoom + 768.0 * 0.5;
        (x as f64, y as f64)
    };
    // Options 格中心 (722, 388)；Exit 格中心 (722, 556)。
    let (ox, oy) = to_win(722, 388);
    let (ex, ey) = to_win(722, 556);
    assert_eq!(hit_action(OriginalScreen::MainMenu, &[], None, (ox, oy), 1024.0, 768.0, false), Some(MenuAction::OpenOptions));
    assert_eq!(hit_action(OriginalScreen::MainMenu, &[], None, (ex, ey), 1024.0, 768.0, false), Some(MenuAction::Exit));
}

#[test]
fn exit_confirm_ok_and_cancel_cells() {
    let cam = ra_layout::ui_layout::shell_fit_camera(1024, 768);
    let to_win = |sx: i32, sy: i32| {
        let x = (sx as f32 - cam.center_x) * cam.zoom + 1024.0 * 0.5;
        let y = (sy as f32 - cam.center_y) * cam.zoom + 768.0 * 0.5;
        (x as f64, y as f64)
    };
    let dlg = ra_layout::ui_layout::exit_confirm_layout(0, 0);
    let ok = dlg.buttons[0];
    let cancel = dlg.buttons[1];
    let (ox, oy) = to_win(ok.x + ok.w / 2, ok.y + ok.h / 2);
    let (cx, cy) = to_win(cancel.x + cancel.w / 2, cancel.y + cancel.h / 2);
    assert_eq!(hit_action(OriginalScreen::ExitConfirm, &[], None, (ox, oy), 1024.0, 768.0, false), Some(MenuAction::ConfirmExit));
    assert_eq!(hit_action(OriginalScreen::ExitConfirm, &[], None, (cx, cy), 1024.0, 768.0, false), Some(MenuAction::Back));
}

#[test]
fn load_screen_hides_hits_while_loading() {
    let loading = hits_for(OriginalScreen::LoadScreen, &[], false);
    assert!(loading.is_empty());
    let failed = hits_for(OriginalScreen::LoadScreen, &[], true);
    let retry = failed.iter().find(|h| h.entry_id == "retry").expect("retry");
    assert!(retry.enabled);
    let cancel = failed.iter().find(|h| h.entry_id == "cancel").expect("cancel");
    assert!(cancel.enabled);
}

#[test]
fn disabled_network_not_hit() {
    // 第二格现为 WWOnline（禁用），壳层约 (722, 262) → 窗口约 (924, 335)。
    let action = hit_action(OriginalScreen::MainMenu, &[], None, (924.0, 335.0), 1024.0, 768.0, false);
    assert_eq!(action, None);
}

#[test]
fn lobby_start_button_is_hit() {
    let maps = vec![BootMapCandidate { file_name: "mp03t4.map".into(), width: 50, height: 50, theater: Theater::Temperate, start_slots: 4 }];
    let layout = skirmish_lobby_layout(0, 0);
    let cell = layout.shell.buttons[0];
    let cx = cell.x + cell.w / 2;
    let cy = cell.y + cell.h / 2;
    let cam = ra_layout::ui_layout::shell_fit_camera(1024, 768);
    let sx = (cx as f32 - cam.center_x) * cam.zoom + 1024.0 * 0.5;
    let sy = (cy as f32 - cam.center_y) * cam.zoom + 768.0 * 0.5;
    let action = hit_action(OriginalScreen::SkirmishLobby, &maps, Some("mp03t4.map"), (sx as f64, sy as f64), 1024.0, 768.0, false);
    assert_eq!(action, Some(MenuAction::StartSkirmish));
}

#[test]
fn hover_index_tracks_enabled_button() {
    assert_eq!(hover_index(OriginalScreen::MainMenu, &[], None, (924.0, 282.0), 1024.0, 768.0, false,), Some(0));
    // 禁用钮仍可悬停（底栏 STT 提示），但不可点击。
    assert_eq!(hover_index(OriginalScreen::MainMenu, &[], None, (924.0, 335.0), 1024.0, 768.0, false,), Some(1));
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

#[test]
fn campaign_side_and_difficulty_are_hit() {
    let cam = ra_layout::ui_layout::shell_fit_camera(1024, 768);
    let to_win = |sx: i32, sy: i32| {
        let x = (sx as f32 - cam.center_x) * cam.zoom + 1024.0 * 0.5;
        let y = (sy as f32 - cam.center_y) * cam.zoom + 768.0 * 0.5;
        (x as f64, y as f64)
    };
    let layout = ra_layout::ui_layout::campaign_layout(0, 0);
    let allied = layout.allied;
    let track = layout.difficulty_track;
    let back = layout.shell.buttons[0];
    let (ax, ay) = to_win(allied.x + allied.w / 2, allied.y + allied.h / 2);
    let (tx, ty) = to_win(track.x + track.w / 2, track.y + track.h / 2);
    let (bx, by) = to_win(back.x + back.w / 2, back.y + back.h / 2);
    assert_eq!(hit_action(OriginalScreen::Campaign, &[], None, (ax, ay), 1024.0, 768.0, false), Some(MenuAction::SelectCampaignAllied));
    assert_eq!(hit_action(OriginalScreen::Campaign, &[], None, (tx, ty), 1024.0, 768.0, false), Some(MenuAction::CycleCampaignDifficulty));
    assert_eq!(hit_action(OriginalScreen::Campaign, &[], None, (bx, by), 1024.0, 768.0, false), Some(MenuAction::Back));
    assert_eq!(campaign_entry_at(ax, ay, 1024.0, 768.0), Some("allied"));
    assert_eq!(campaign_entry_at(tx, ty, 1024.0, 768.0), Some("difficulty"));
}

#[test]
fn choose_map_use_and_list_row_are_hit() {
    let maps = vec![BootMapCandidate { file_name: "mp03t4.map".into(), width: 50, height: 50, theater: Theater::Temperate, start_slots: 4 }];
    let cam = ra_layout::ui_layout::shell_fit_camera(1024, 768);
    let to_win = |sx: i32, sy: i32| {
        let x = (sx as f32 - cam.center_x) * cam.zoom + 1024.0 * 0.5;
        let y = (sy as f32 - cam.center_y) * cam.zoom + 768.0 * 0.5;
        (x as f64, y as f64)
    };
    let layout = ra_layout::ui_layout::choose_map_layout(0, 0);
    let use_map = layout.shell.buttons[0];
    let (ux, uy) = to_win(use_map.x + use_map.w / 2, use_map.y + use_map.h / 2);
    assert_eq!(hit_action(OriginalScreen::ChooseMap, &maps, Some("mp03t4.map"), (ux, uy), 1024.0, 768.0, false), Some(MenuAction::UseMap));
    let (mx, my) = to_win(layout.map_list.x + 8, layout.map_list.y + 8);
    assert_eq!(
        hit_action(OriginalScreen::ChooseMap, &maps, Some("mp03t4.map"), (mx, my), 1024.0, 768.0, false),
        Some(MenuAction::SelectMap(0))
    );
}
