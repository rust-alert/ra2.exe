//! 放弃确认 / 局内选项 layout 冒烟。

use ra_layout::{
    rect_px_from_snapshot, solve_battle_abort_confirm, solve_battle_in_game_options, solve_battle_pause,
};

#[test]
fn abort_confirm_is_centered_card_family_not_sidebttn_rail() {
    let pause = solve_battle_pause();
    let abort = solve_battle_abort_confirm();
    let opts = solve_battle_in_game_options();
    assert!(pause.get("sidebar").is_none());
    assert!(abort.get("panel_top").is_none());
    assert!(opts.get("mnscrnl").is_none());
    assert!(pause.get("card").is_some());
    assert!(abort.get("card").is_some());
    let leave = rect_px_from_snapshot(&abort, "leave");
    let cancel = rect_px_from_snapshot(&abort, "cancel");
    assert_eq!(leave.w, 240);
    assert_eq!(leave.h, 40);
    assert_eq!(leave.x, cancel.x);
    assert!(leave.x > 200, "leave must be centered, not sidebar-pinned");
    // 局内选项仍走 0xBBB SIDEBTTN 右缘。
    let back = rect_px_from_snapshot(&opts, "back");
    assert_eq!(back.w, 125);
    assert_eq!(back.x, 800 - 147);
    assert_ne!(back.x, leave.x);
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
