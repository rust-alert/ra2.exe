//! 装载页命中消费 `load_screen_layout_tree` snapshot。

use ra_layout::load_screen_layout;
use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    ui_hit::{hit_action, hits_for},
};

#[test]
fn load_screen_hits_use_content_snapshot() {
    let maps = [];
    assert!(hits_for(OriginalScreen::LoadScreen, &maps, 0, false).is_empty());

    let hits = hits_for(OriginalScreen::LoadScreen, &maps, 0, true);
    assert!(hits.iter().any(|h| h.entry_id == "retry"));
    assert!(hits.iter().any(|h| h.entry_id == "cancel"));

    let layout = load_screen_layout(800, 600);
    let retry = layout.buttons[0];
    let cancel = layout.buttons[1];
    assert_eq!(
        hit_action(
            OriginalScreen::LoadScreen,
            &maps,
            0,
            None,
            ((retry.x + retry.w / 2) as f64, (retry.y + retry.h / 2) as f64),
            800.0,
            600.0,
            true,
        ),
        Some(MenuAction::RetryLoad)
    );
    assert_eq!(
        hit_action(
            OriginalScreen::LoadScreen,
            &maps,
            0,
            None,
            ((cancel.x + cancel.w / 2) as f64, (cancel.y + cancel.h / 2) as f64),
            800.0,
            600.0,
            true,
        ),
        Some(MenuAction::CancelLoad)
    );
}
