//! 战役页右栏「上一页」命中改读 chrome snapshot。

use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    ui_hit::hit_action,
};

#[test]
fn campaign_back_hit_uses_right_rail_snapshot() {
    let maps = [];
    assert_eq!(
        hit_action(
            OriginalScreen::Campaign,
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
