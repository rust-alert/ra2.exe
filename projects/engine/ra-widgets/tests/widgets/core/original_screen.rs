//! 自顶层 `original_screen.rs`。

use ra_widgets::original_screen::OriginalScreen;

#[test]
fn emulate_alias_skirmish_and_main() {
    assert_eq!(OriginalScreen::parse_emulate_alias("skirmish").unwrap(), OriginalScreen::SkirmishLobby);
    assert_eq!(OriginalScreen::parse_emulate_alias("SKIRMISH_LOBBY").unwrap(), OriginalScreen::SkirmishLobby);
    assert_eq!(OriginalScreen::parse_emulate_alias("lobby").unwrap(), OriginalScreen::SkirmishLobby);
    assert_eq!(OriginalScreen::parse_emulate_alias("main").unwrap(), OriginalScreen::MainMenu);
    assert_eq!(OriginalScreen::parse_emulate_alias("single_player").unwrap(), OriginalScreen::SinglePlayerMenu);
    assert_eq!(OriginalScreen::parse_emulate_alias("campaign").unwrap(), OriginalScreen::Campaign);
    assert_eq!(OriginalScreen::parse_emulate_alias("choose_map").unwrap(), OriginalScreen::ChooseMap);
}

#[test]
fn emulate_alias_rejects_battle_and_unknown() {
    assert!(OriginalScreen::parse_emulate_alias("battle").unwrap_err().contains("对局会话"));
    assert!(OriginalScreen::parse_emulate_alias("load_screen").unwrap_err().contains("对局会话"));
    assert!(OriginalScreen::parse_emulate_alias("nope").unwrap_err().contains("未知"));
    assert!(OriginalScreen::parse_emulate_alias("  ").unwrap_err().contains("空"));
}
