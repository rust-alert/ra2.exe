//! 集成测试：原 `src/ui_slots.rs` 内联测试迁出。

use ra_widgets::{menu_action::MenuAction, original_screen::OriginalScreen, ui_slots::*};
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
    assert!(page.panels.iter().any(|p| p.shp == "sdwrnanm.shp"));
    assert_eq!(page.fonts, &["game.fnt"]);
}

#[test]
fn single_player_order_is_campaign_load_skirmish_back() {
    let page = slots_for(OriginalScreen::SinglePlayerMenu).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["campaign", "load", "skirmish", "back"]);
    assert!(page.buttons[0].enabled);
    assert!(matches!(page.buttons[0].action, MenuAction::OpenCampaign));
    assert!(!page.buttons[1].enabled);
    assert!(matches!(page.buttons[1].action, MenuAction::Noop));
    assert!(page.buttons[2].enabled);
    assert!(matches!(page.buttons[2].action, MenuAction::OpenSkirmish));
    assert!(page.buttons[3].enabled);
    assert!(matches!(page.buttons[3].action, MenuAction::Back));
    assert!(page.has_any_asset_name());
    assert_eq!(page.background_shp, Some("mnscrnl.shp"));
    assert_eq!(page.buttons[2].normal_frame, Some(2));
    assert_eq!(page.buttons[2].pressed_frame, Some(4));
}

#[test]
fn campaign_declares_three_side_panels_and_rail() {
    let page = slots_for(OriginalScreen::Campaign).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["back"]);
    assert!(page.buttons[0].enabled);
    assert!(matches!(page.buttons[0].action, MenuAction::Back));
    assert_eq!(page.movie_bik, None);
    assert_eq!(page.background_shp, Some("fsbkgdlg.shp"));
    assert_eq!(page.background_pal, Some("fsscrn.pal"));
    assert!(page.panels.iter().any(|p| p.shp == "fsalg.shp" && p.pal == "fsscrn.pal"));
    assert!(page.panels.iter().any(|p| p.shp == "fsbclg.shp" && p.pal == "fsscrn.pal"));
    assert!(page.panels.iter().any(|p| p.shp == "fsslg.shp" && p.pal == "fsscrn.pal"));
    assert!(page.panels.iter().any(|p| p.shp == "sdwrnanm.shp"));
    assert!(page.has_any_asset_name());
}

#[test]
fn load_screen_exposes_retry_and_cancel_slots() {
    let page = slots_for(OriginalScreen::LoadScreen).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["retry", "cancel"]);
    let retry = page.buttons.iter().find(|b| b.entry_id == "retry").unwrap();
    assert!(retry.enabled);
    assert!(matches!(retry.action, MenuAction::RetryLoad));
    assert!(retry.anim_shp.is_some());
    let cancel = page.buttons.iter().find(|b| b.entry_id == "cancel").unwrap();
    assert!(cancel.enabled);
    assert!(matches!(cancel.action, MenuAction::CancelLoad));
    assert_eq!(page.background_shp, Some("ls800ustates.shp"));
    assert_eq!(page.background_pal, Some("mpls.pal"));
    assert!(page.panels.iter().any(|p| p.shp == "progbarm.shp"));
    assert!(page.has_any_asset_name());
    assert_eq!(page.fonts, &["game.fnt"]);
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
    assert!(page.panels.iter().any(|p| p.shp == "sdwrnanm.shp"));
    assert_eq!(page.fonts, &["game.fnt"]);
}

#[test]
fn skirmish_lobby_exposes_start_choose_map_back() {
    let page = slots_for(OriginalScreen::SkirmishLobby).unwrap();
    let ids: Vec<_> = page.buttons.iter().map(|b| b.entry_id).collect();
    assert_eq!(ids, ["start", "choose_map", "back"]);
    assert!(matches!(page.buttons[0].action, MenuAction::StartSkirmish));
    assert!(matches!(page.buttons[1].action, MenuAction::ChooseMap));
    assert!(matches!(page.buttons[2].action, MenuAction::Back));
    assert!(page.has_any_asset_name());
    assert_eq!(page.background_shp, Some("mnscrnl.shp"));
    assert_eq!(page.buttons[0].normal_frame, Some(2));
    assert_eq!(page.buttons[0].pressed_frame, Some(4));
    assert!(page.panels.iter().any(|p| p.shp == "sdtp.shp"));
    assert!(page.panels.iter().any(|p| p.shp == "sdmpbtn.shp"));
    assert!(!page.panels.iter().any(|p| p.shp == "sdwrnanm.shp"));
    assert_eq!(page.fonts, &["game.fnt"]);
}

#[test]
fn splash_has_no_menu_shell_slots() {
    // 进程启动闪屏不走菜单槽；资源由 `startup_splash` 绑定 `GLSS`/`GLSL`。
    assert!(slots_for(OriginalScreen::Splash).is_none());
}
