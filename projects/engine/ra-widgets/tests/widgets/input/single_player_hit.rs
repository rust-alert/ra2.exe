//! 单人页命中消费 solve_shell_page snapshot。

use ra_widgets::{input::hit::hit_action, menu_action::MenuAction, original_screen::OriginalScreen};

#[test]
fn single_player_hits_use_shell_page_snapshot() {
    let maps = [];
    assert_eq!(
        hit_action(OriginalScreen::SinglePlayerMenu, &maps, 0, None, (722.0, 220.0), 800.0, 600.0, 0, false,),
        Some(MenuAction::OpenCampaign)
    );
    assert_eq!(
        hit_action(OriginalScreen::SinglePlayerMenu, &maps, 0, None, (722.0, 304.0), 800.0, 600.0, 0, false,),
        Some(MenuAction::OpenSkirmish)
    );
    assert_eq!(hit_action(OriginalScreen::SinglePlayerMenu, &maps, 0, None, (722.0, 556.0), 800.0, 600.0, 0, false,), Some(MenuAction::Back));
}
