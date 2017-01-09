//! 暂停菜单专属 layout：战术区 + 右 hub 壳槽 + SIDEBTTN 列表（同一棵树）。

use ra_layout::{
    BATTLE_PAUSE_MENU_BUTTON_IDS, BATTLE_PAUSE_RAIL_W, COMMAND_BAR_H, rect_px_from_snapshot, solve_battle_pause,
    solve_battle_pause_at,
};

#[test]
fn battle_pause_tree_owns_hub_slots_not_main_shell() {
    let snap = solve_battle_pause();
    assert!(snap.get("dim").is_some());
    assert!(snap.get("background").is_some());
    assert!(snap.get("sidebar").is_some());
    assert!(snap.get("radar").is_some());
    assert!(snap.get("list_band").is_some());
    assert!(snap.get("tab00").is_some());
    assert!(snap.get("tab03").is_some());
    assert!(snap.get("addon").is_some());
    assert!(snap.get("command_bar").is_some());
    assert!(snap.get("game_controls").is_some());
    assert!(snap.get("resume").is_some());
    assert!(snap.get("card").is_none());
    assert!(snap.get("sdtp").is_none());
    assert!(snap.get("panel_top").is_none());
}

#[test]
fn battle_pause_has_six_button_ids() {
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS.len(), 6);
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[0], "game_controls");
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[1], "load");
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[5], "resume");
}

#[test]
fn battle_pause_hub_and_packed_list_at_800x600() {
    let snap = solve_battle_pause_at(800, 600);
    let background = rect_px_from_snapshot(&snap, "background");
    assert_eq!(background.w, 800 - BATTLE_PAUSE_RAIL_W as i32);
    assert_eq!(background.h, 600 - COMMAND_BAR_H);
    assert_eq!(background.x, 0);
    assert_eq!(background.y, 0);

    let sidebar = rect_px_from_snapshot(&snap, "sidebar");
    assert_eq!(sidebar.w, BATTLE_PAUSE_RAIL_W as i32);
    assert_eq!(sidebar.x, 800 - BATTLE_PAUSE_RAIL_W as i32);

    let list_band = rect_px_from_snapshot(&snap, "list_band");
    let game_controls = rect_px_from_snapshot(&snap, "game_controls");
    assert_eq!(game_controls.w, 125);
    assert_eq!(game_controls.h, 25);
    assert_eq!(game_controls.x, sidebar.x + 21);
    assert_eq!(game_controls.y, list_band.y);

    let load = rect_px_from_snapshot(&snap, "load");
    assert_eq!(load.y, game_controls.y + 25, "upper list should pack tightly");

    let resume = rect_px_from_snapshot(&snap, "resume");
    assert_eq!(resume.y + resume.h, list_band.y + list_band.h, "resume sits on list_band bottom");
}
