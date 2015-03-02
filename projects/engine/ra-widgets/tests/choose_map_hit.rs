//! 选图命中改为消费 `0x6B` snapshot。

use ra_map::BootMapCandidate;
use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    ui_hit::{hit_action, hits_for},
};

#[test]
fn choose_map_hits_use_snapshot_button_rects() {
    let maps: Vec<BootMapCandidate> = Vec::new();
    let hits = hits_for(OriginalScreen::ChooseMap, &maps, false);
    assert!(hits.iter().any(|h| h.entry_id == "use_map"));
    assert!(hits.iter().any(|h| h.entry_id == "cancel"));

    // 800×600 窗口下，点 use_map 格中心 (644+78, 199+21) ≈ (722, 220)。
    let action = hit_action(
        OriginalScreen::ChooseMap,
        &maps,
        None,
        (722.0, 220.0),
        800.0,
        600.0,
        false,
    );
    assert_eq!(action, Some(MenuAction::UseMap));

    let cancel = hit_action(
        OriginalScreen::ChooseMap,
        &maps,
        None,
        (722.0, 556.0),
        800.0,
        600.0,
        false,
    );
    assert_eq!(cancel, Some(MenuAction::Back));
}
