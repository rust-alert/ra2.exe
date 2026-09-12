//! 暂停二级页：放弃确认 / 局内选项。

use ra_layout::{BATTLE_ABORT_CONFIRM_BUTTON_IDS, rect_px_from_snapshot};
use ra_widgets::{
    battle_abort_confirm::{BattleAbortConfirmHit, button_rects as abort_rects, hit_at as abort_hit},
    battle_in_game_options::{
        BattleInGameOptionsHit, BattleInGameOptionsState, button_rects as opts_rects, hit_at as opts_hit, options_snapshot,
    },
    battle_pause_layer::{BattlePauseLayer, EscapeRoute},
    compose::{compose_battle_abort_confirm_overlay, compose_battle_in_game_options_overlay},
};

#[test]
fn abort_confirm_hits_leave_and_cancel() {
    let rects = abort_rects(800, 600);
    assert_eq!(BATTLE_ABORT_CONFIRM_BUTTON_IDS.len(), 2);
    let leave = rects[0];
    let cancel = rects[1];
    assert!(cancel.y > leave.y);
    assert_eq!(abort_hit(800, 600, leave.x + 4, leave.y + 4), Some(BattleAbortConfirmHit::Leave));
    assert_eq!(abort_hit(800, 600, cancel.x + 4, cancel.y + 4), Some(BattleAbortConfirmHit::Cancel));
}

#[test]
fn compose_abort_confirm_dims_page() {
    let page = compose_battle_abort_confirm_overlay(800, 600, None, Some("leave"), None, None, None).unwrap();
    assert_eq!(page.width(), 800);
    assert_eq!(page.height(), 600);
    assert!(page.as_raw()[3] > 0);
}

#[test]
fn in_game_options_hits_back_and_tracks() {
    let rects = opts_rects(800, 600);
    let back = rects[2];
    assert_eq!(opts_hit(800, 600, back.x + 4, back.y + 4), Some(BattleInGameOptionsHit::Back));
    let track = rect_px_from_snapshot(&options_snapshot(800, 600), "track_game_speed");
    assert_eq!(opts_hit(800, 600, track.x + 4, track.y + 4), Some(BattleInGameOptionsHit::TrackGameSpeed));
}

#[test]
fn compose_in_game_options_shows_stub_notice_alpha() {
    let state = BattleInGameOptionsState::default();
    let page = compose_battle_in_game_options_overlay(800, 600, &state, None, None, None, None, None, Some("暂未实现")).unwrap();
    assert_eq!(page.width(), 800);
    assert!(page.as_raw()[3] > 0);
}

#[test]
fn pause_layer_escape_routes_match_vera() {
    assert_eq!(BattlePauseLayer::Menu.on_escape(), EscapeRoute::ResumeMission);
    assert_eq!(BattlePauseLayer::AbortConfirm.on_escape(), EscapeRoute::ResumeMission);
    assert_eq!(BattlePauseLayer::InGameOptions.on_escape(), EscapeRoute::ToLayer(BattlePauseLayer::Menu));
}
