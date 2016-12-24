//! 暂停菜单专属 layout（不是 battle HUD / 主菜单；右缘 SIDEBTTN）。

use ra_layout::{BATTLE_PAUSE_MENU_BUTTON_IDS, rect_px_from_snapshot, solve_battle_pause, solve_battle_pause_at};

#[test]
fn battle_pause_is_not_battle_hud_or_main_shell() {
    let snap = solve_battle_pause();
    assert!(snap.get("dim").is_some());
    assert!(snap.get("background").is_some());
    assert!(snap.get("load").is_some());
    assert!(snap.get("resume").is_some());
    assert!(snap.get("card").is_none());
    assert!(snap.get("sidebar").is_none());
    assert!(snap.get("radar").is_none());
    assert!(snap.get("cameo_band").is_none());
    assert!(snap.get("panel_top").is_none());
    assert!(snap.get("sdtp").is_none());
}

#[test]
fn battle_pause_has_six_button_ids() {
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS.len(), 6);
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[0], "load");
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[5], "resume");
}

#[test]
fn battle_pause_background_and_buttons_at_800x600() {
    let snap = solve_battle_pause_at(800, 600);
    let background = rect_px_from_snapshot(&snap, "background");
    assert_eq!(background.w, 632);
    assert_eq!(background.h, 568);
    let load = rect_px_from_snapshot(&snap, "load");
    assert_eq!(load.w, 125);
    assert_eq!(load.h, 25);
    assert_eq!(load.x, 800 - 147);
    let resume = rect_px_from_snapshot(&snap, "resume");
    assert!(resume.y > load.y);
}
