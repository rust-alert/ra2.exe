//! 单人页命中改为消费右栏 chrome snapshot。

use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    ui_hit::hit_action,
};

#[test]
fn single_player_hits_use_right_rail_snapshot() {
    let maps = [];
    assert_eq!(
        hit_action(
            OriginalScreen::SinglePlayerMenu,
            &maps,
            None,
            (722.0, 220.0),
            800.0,
            600.0,
            false,
        ),
        Some(MenuAction::OpenCampaign)
    );
    assert_eq!(
        hit_action(
            OriginalScreen::SinglePlayerMenu,
            &maps,
            None,
            (722.0, 304.0),
            800.0,
            600.0,
            false,
        ),
        Some(MenuAction::OpenSkirmish)
    );
    assert_eq!(
        hit_action(
            OriginalScreen::SinglePlayerMenu,
            &maps,
            None,
            (722.0, 556.0),
            800.0,
            600.0,
            false,
        ),
        Some(MenuAction::Back)
    );
}
