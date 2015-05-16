//! 遭遇战大厅命中改为消费 `0x102` snapshot。

use ra_map::BootMapCandidate;
use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    ui_hit::hit_action,
};

#[test]
fn skirmish_lobby_hits_use_snapshot_button_rects() {
    let maps: Vec<BootMapCandidate> = Vec::new();
    // start 格中心 ≈ (722, 262)；choose_map ≈ (722, 304)；back ≈ (722, 556)。
    assert_eq!(
        hit_action(
            OriginalScreen::SkirmishLobby,
            &maps,
            0,
            None,
            (722.0, 262.0),
            800.0,
            600.0,
            false,
        ),
        Some(MenuAction::StartSkirmish)
    );
    assert_eq!(
        hit_action(
            OriginalScreen::SkirmishLobby,
            &maps,
            0,
            None,
            (722.0, 304.0),
            800.0,
            600.0,
            false,
        ),
        Some(MenuAction::ChooseMap)
    );
    assert_eq!(
        hit_action(
            OriginalScreen::SkirmishLobby,
            &maps,
            0,
            None,
            (722.0, 556.0),
            800.0,
            600.0,
            false,
        ),
        Some(MenuAction::Back)
    );
}
