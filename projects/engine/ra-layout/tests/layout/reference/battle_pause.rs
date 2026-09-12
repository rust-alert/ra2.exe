//! 暂停菜单专属 layout（不是 battle HUD / 主菜单）。

use ra_layout::{rect_px_from_snapshot, solve_battle_pause, solve_battle_pause_at};

#[test]
fn battle_pause_is_not_battle_hud_or_main_shell() {
    let snap = solve_battle_pause();
    assert!(snap.get("dim").is_some());
    assert!(snap.get("options").is_some());
    assert!(snap.get("resume").is_some());
    // 禁止混入对局 HUD / 主菜单 chrome 叶。
    assert!(snap.get("sidebar").is_none());
    assert!(snap.get("radar").is_none());
    assert!(snap.get("cameo_band").is_none());
    assert!(snap.get("panel_top").is_none());
    assert!(snap.get("sdtp").is_none());
}

#[test]
fn battle_pause_buttons_pin_sidebttn_at_800x600() {
    let snap = solve_battle_pause_at(800, 600);
    let options = rect_px_from_snapshot(&snap, "options");
    assert_eq!(options.w, 125);
    assert_eq!(options.h, 25);
    assert_eq!(options.x, 800 - 147);
    let resume = rect_px_from_snapshot(&snap, "resume");
    assert!(resume.y > options.y);
}
