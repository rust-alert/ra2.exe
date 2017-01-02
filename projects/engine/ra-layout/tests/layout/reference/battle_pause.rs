//! 暂停菜单专属 layout（不是 battle HUD / 主菜单；左区 `bkgd*` + 右缘 `SIDEBTTN`）。
//! 右侧金属壳与底边命令条由暂停态 HUD chrome 垫底，本 snapshot 不声明 `sidebar`/`addon`。

use ra_layout::{
    BATTLE_PAUSE_MENU_BUTTON_IDS, BATTLE_PAUSE_RAIL_W, COMMAND_BAR_H, rect_px_from_snapshot, solve_battle_pause,
    solve_battle_pause_at,
};

#[test]
fn battle_pause_is_not_battle_hud_or_main_shell() {
    let snap = solve_battle_pause();
    assert!(snap.get("dim").is_some());
    assert!(snap.get("background").is_some());
    assert!(snap.get("rail").is_some());
    assert!(snap.get("game_controls").is_some());
    assert!(snap.get("resume").is_some());
    assert!(snap.get("card").is_none());
    assert!(snap.get("sidebar").is_none());
    assert!(snap.get("radar").is_none());
    assert!(snap.get("cameo_band").is_none());
    assert!(snap.get("addon").is_none());
    assert!(snap.get("panel_top").is_none());
    assert!(snap.get("sdtp").is_none());
}

#[test]
fn battle_pause_has_six_button_ids() {
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS.len(), 6);
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[0], "game_controls");
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[1], "load");
    assert_eq!(BATTLE_PAUSE_MENU_BUTTON_IDS[5], "resume");
}

#[test]
fn battle_pause_background_rail_and_buttons_at_800x600() {
    let snap = solve_battle_pause_at(800, 600);
    let background = rect_px_from_snapshot(&snap, "background");
    assert_eq!(background.w, 800 - BATTLE_PAUSE_RAIL_W as i32);
    // 底边留给命令条空轨，勿盖住。
    assert_eq!(background.h, 600 - COMMAND_BAR_H);
    assert_eq!(background.x, 0);
    assert_eq!(background.y, 0);
    let rail = rect_px_from_snapshot(&snap, "rail");
    assert_eq!(rail.w, BATTLE_PAUSE_RAIL_W as i32);
    assert_eq!(rail.h, 600);
    assert_eq!(rail.x, 800 - BATTLE_PAUSE_RAIL_W as i32);
    let game_controls = rect_px_from_snapshot(&snap, "game_controls");
    assert_eq!(game_controls.w, 125);
    assert_eq!(game_controls.h, 25);
    assert_eq!(game_controls.x, 800 - 147);
    let resume = rect_px_from_snapshot(&snap, "resume");
    assert!(resume.y > game_controls.y);
}
