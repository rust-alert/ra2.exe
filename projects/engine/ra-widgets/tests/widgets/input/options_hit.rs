//! 选项页命中消费 solve_options_page snapshot。

use ra_widgets::{input::hit::hit_action, menu_action::MenuAction, original_screen::OriginalScreen};

#[test]
fn options_hits_use_shell_page_snapshot() {
    let maps = [];
    assert_eq!(hit_action(OriginalScreen::Options, &maps, 0, None, (722.0, 220.0), 800.0, 600.0, 0, false,), Some(MenuAction::Noop));
    assert_eq!(hit_action(OriginalScreen::Options, &maps, 0, None, (722.0, 262.0), 800.0, 600.0, 0, false,), Some(MenuAction::OpenNetwork));
    assert_eq!(hit_action(OriginalScreen::Options, &maps, 0, None, (722.0, 556.0), 800.0, 600.0, 0, false,), Some(MenuAction::OptionsAccept));
}
