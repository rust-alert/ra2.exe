//! 单人页命中消费 `shell_page_layout_tree` snapshot。

use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    ui_hit::hit_action,
};

#[test]
fn single_player_hits_use_shell_page_snapshot() {
    let maps = [];
    assert_eq!(
        hit_action(
            OriginalScreen::SinglePlayerMenu,
            &maps,
            0,
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
            0,
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
