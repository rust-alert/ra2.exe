//! 集成测试：原 `src/ui_slots.rs` 内联测试迁出。

use ra_desktop::{menu_action::MenuAction, screen::OriginalScreen, ui_slots::*};
#[test]
fn main_menu_entries_match_expected_ids() {
    let page = slots_for(OriginalScreen::MainMenu).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["single_player", "ww_online", "network", "movies", "options", "exit"]);
    assert!(!page.buttons[1].enabled);
    assert!(!page.buttons[2].enabled);
    assert!(!page.buttons[3].enabled);
    assert!(page.has_any_asset_name());
    assert_eq!(page.background_shp, Some("mnscrnl.shp"));
    assert_eq!(page.background_pal, Some("shell.pal"));
    assert_eq!(page.movie_bik, Some("ra2ts_l.bik"));
    assert_eq!(page.buttons[0].hover_frame, None);
    assert_eq!(page.buttons[0].normal_frame, Some(2));
    assert_eq!(page.buttons[0].pressed_frame, Some(4));
    assert!(page.panels.iter().any(|p| p.shp == "sdtp.shp"));
    assert_eq!(page.fonts, &["game.fnt"]);
}

#[test]
fn single_player_keeps_disabled_campaign_slot() {
    let page = slots_for(OriginalScreen::SinglePlayerMenu).unwrap();
    assert_eq!(page.buttons[0].entry_id, "campaign");
    assert!(!page.buttons[0].enabled);
    assert!(matches!(page.buttons[0].action, MenuAction::Noop));
    assert!(matches!(page.buttons[2].action, MenuAction::Noop));
    assert!(page.buttons.iter().any(|b| b.entry_id == "skirmish" && b.enabled));
    assert!(page.has_any_asset_name());
    assert_eq!(page.background_shp, Some("mnscrnl.shp"));
    assert_eq!(page.buttons[1].normal_frame, Some(2));
    assert_eq!(page.buttons[1].pressed_frame, Some(4));
}

#[test]
fn load_screen_exposes_retry_and_cancel_slots() {
    let page = slots_for(OriginalScreen::LoadScreen).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["loading", "retry", "cancel"]);
    let retry = page.buttons.iter().find(|b| b.entry_id == "retry").unwrap();
    assert!(retry.enabled);
    assert!(matches!(retry.action, MenuAction::RetryLoad));
    let cancel = page.buttons.iter().find(|b| b.entry_id == "cancel").unwrap();
    assert!(cancel.enabled);
    assert!(matches!(cancel.action, MenuAction::CancelLoad));
}

#[test]
fn options_rail_accept_cancel_main_menu() {
    let page = slots_for(OriginalScreen::Options).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["accept", "cancel", "main_menu"]);
    assert!(page.buttons[0].enabled);
    assert!(matches!(page.buttons[0].action, MenuAction::OptionsAccept));
    assert!(page.buttons[1].enabled);
    assert!(matches!(page.buttons[1].action, MenuAction::OptionsCancel));
    assert!(page.buttons[2].enabled);
    assert!(matches!(page.buttons[2].action, MenuAction::Back));
    assert!(page.has_any_asset_name());
    assert_eq!(page.background_shp, Some("mnscrnl.shp"));
    assert_eq!(page.background_pal, Some("shell.pal"));
    assert_eq!(page.movie_bik, Some("ra2ts_l.bik"));
    assert_eq!(page.buttons[2].normal_frame, Some(2));
    assert_eq!(page.buttons[2].pressed_frame, Some(4));
    assert!(page.panels.iter().any(|p| p.shp == "sdtp.shp"));
    assert_eq!(page.fonts, &["game.fnt"]);
}

#[test]
fn skirmish_lobby_exposes_side_and_difficulty() {
    let page = slots_for(OriginalScreen::SkirmishLobby).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["side", "difficulty", "start", "back"]);
    assert!(matches!(page.buttons[0].action, MenuAction::CycleSide));
    assert!(matches!(page.buttons[1].action, MenuAction::CycleDifficulty));
    assert!(page.has_any_asset_name());
    assert_eq!(page.background_shp, Some("mnscrnl.shp"));
    assert_eq!(page.buttons[0].normal_frame, Some(2));
    assert_eq!(page.buttons[0].pressed_frame, Some(4));
    assert!(page.panels.iter().any(|p| p.shp == "sdtp.shp"));
    assert_eq!(page.fonts, &["game.fnt"]);
}

#[test]
fn splash_declares_title_pcx() {
    let page = slots_for(OriginalScreen::Splash).unwrap();
    assert_eq!(page.background_pcx, Some("title.pcx"));
    assert!(page.has_any_asset_name());
    assert!(page.buttons.is_empty());
}
