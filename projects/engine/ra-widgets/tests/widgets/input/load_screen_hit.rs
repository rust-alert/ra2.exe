//! 装载页命中消费 solve_load_screen snapshot。

use ra_layout::{LOAD_SCREEN_BUTTON_IDS, rect_px_from_snapshot, solve_load_screen};
use ra_widgets::{
    input::hit::{hit_action, hits_for},
    menu_action::MenuAction,
    original_screen::OriginalScreen,
};

#[test]
fn load_screen_hits_use_content_snapshot() {
    let maps = [];
    assert!(hits_for(OriginalScreen::LoadScreen, &maps, 0, 0, false).is_empty());

    let hits = hits_for(OriginalScreen::LoadScreen, &maps, 0, 0, true);
    assert!(hits.iter().any(|h| h.entry_id == "retry"));
    assert!(hits.iter().any(|h| h.entry_id == "cancel"));

    let snap = solve_load_screen();
    let retry = rect_px_from_snapshot(&snap, LOAD_SCREEN_BUTTON_IDS[0]);
    let cancel = rect_px_from_snapshot(&snap, LOAD_SCREEN_BUTTON_IDS[1]);
    assert_eq!(
        hit_action(
            OriginalScreen::LoadScreen,
            &maps,
            0,
            None,
            ((retry.x + retry.w / 2) as f64, (retry.y + retry.h / 2) as f64),
            800.0,
            600.0,
            0,
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
            0,
            true,
        ),
        Some(MenuAction::CancelLoad)
    );
}
