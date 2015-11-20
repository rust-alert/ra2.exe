//! 主菜单命中消费 solve_shell_page snapshot。

use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    input::hit::{hit_action, hover_index},
};

#[test]
fn main_menu_hits_use_shell_page_snapshot() {
    let maps = [];
    assert_eq!(
        hit_action(
            OriginalScreen::MainMenu,
            &maps,
            0,
            None,
            (722.0, 220.0),
            800.0,
            600.0,
            0, false,
        ),
        Some(MenuAction::OpenSinglePlayer)
    );
    assert_eq!(
        hit_action(
            OriginalScreen::MainMenu,
            &maps,
            0,
            None,
            (722.0, 556.0),
            800.0,
            600.0,
            0, false,
        ),
        Some(MenuAction::Exit)
    );
    // 禁用的 network 格仍可悬停（下标 2），点击无动作。
    assert_eq!(
        hover_index(
            OriginalScreen::MainMenu,
            &maps,
            0,
            None,
            (722.0, 304.0),
            800.0,
            600.0,
            0, false,
        ),
        Some(2)
    );
    assert_eq!(
        hit_action(
            OriginalScreen::MainMenu,
            &maps,
            0,
            None,
            (722.0, 304.0),
            800.0,
            600.0,
            0, false,
        ),
        None
    );
}
