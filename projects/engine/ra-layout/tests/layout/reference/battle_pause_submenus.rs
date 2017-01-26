//! 放弃确认 / 局内选项 layout 冒烟。

use ra_layout::{rect_px_from_snapshot, solve_battle_abort_confirm, solve_battle_hud, solve_battle_in_game_options, solve_battle_pause};

#[test]
fn abort_confirm_and_options_share_sidebttn_rail() {
    let pause = solve_battle_pause();
    let abort = solve_battle_abort_confirm();
    let opts = solve_battle_in_game_options();
    let hud = solve_battle_hud(800, 600);
    assert!(pause.get("list_band").is_some());
    assert!(pause.get("sidebar").is_some());
    assert!(pause.get("card").is_none());
    assert!(abort.get("panel_top").is_none());
    assert!(abort.get("background").is_some());
    assert!(abort.get("list_band").is_some());
    assert!(abort.get("sidebar").is_some());
    assert!(abort.get("rail").is_some());
    assert_eq!(rect_px_from_snapshot(&pause, "background"), rect_px_from_snapshot(&abort, "background"),);
    assert_eq!(rect_px_from_snapshot(&pause, "list_band"), rect_px_from_snapshot(&abort, "list_band"),);
    assert_eq!(rect_px_from_snapshot(&pause, "rail"), rect_px_from_snapshot(&abort, "rail"),);
    assert_eq!(rect_px_from_snapshot(&pause, "tab00"), rect_px_from_snapshot(&hud, "tab00"), "pause hub tabs share HUD geometry");
    assert_eq!(rect_px_from_snapshot(&pause, "tab00"), rect_px_from_snapshot(&abort, "tab00"),);
    assert_eq!(rect_px_from_snapshot(&pause, "tab00"), rect_px_from_snapshot(&opts, "tab00"),);
    assert!(opts.get("mnscrnl").is_none());
    assert!(opts.get("list_band").is_some());
    assert!(opts.get("sidebar").is_some());
    assert_eq!(rect_px_from_snapshot(&pause, "list_band"), rect_px_from_snapshot(&opts, "list_band"),);
    let leave = rect_px_from_snapshot(&abort, "leave");
    let cancel = rect_px_from_snapshot(&abort, "cancel");
    assert_eq!(leave.w, 125);
    assert_eq!(cancel.h, 25);
    assert_eq!(leave.x, cancel.x);
    assert_eq!(leave.x, 800 - 147);
    let back = rect_px_from_snapshot(&opts, "back");
    assert_eq!(back.w, 125);
    assert_eq!(back.x, leave.x);
}

#[test]
fn in_game_options_exposes_tracks_and_checks() {
    let snap = solve_battle_in_game_options();
    for id in [
        "track_game_speed",
        "track_scroll_rate",
        "check_target_lines",
        "check_show_hidden",
        "check_tooltips",
        "footer",
        "sound",
        "keyboard",
        "back",
    ] {
        assert!(snap.get(id).is_some(), "missing {id}");
    }
}
