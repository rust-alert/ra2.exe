//! 选图命中改为消费 `0x6B` snapshot。

use ra_map::{BootMapCandidate, Theater};
use ra_widgets::{
    menu_action::MenuAction,
    original_screen::OriginalScreen,
    input::hit::{hit_action, hits_for},
};

fn sample_map(name: &str) -> BootMapCandidate {
    BootMapCandidate {
        file_name: name.into(),
        name_csf: format!("DESC:{}", name.trim_end_matches(".map").to_ascii_uppercase()),
        width: 50,
        height: 50,
        theater: Theater::Temperate,
        start_slots: 4,
        game_modes: Vec::new(),
    }
}

#[test]
fn choose_map_hits_use_snapshot_button_rects() {
    let maps: Vec<BootMapCandidate> = Vec::new();
    let hits = hits_for(OriginalScreen::ChooseMap, &maps, 0, 0, false);
    assert!(hits.iter().any(|h| h.entry_id == "use_map"));
    assert!(hits.iter().any(|h| h.entry_id == "cancel"));
    assert!(hits.iter().any(|h| h.entry_id == "create_random" && h.enabled));

    // 800×600 窗口下，点 use_map 格中心 (644+78, 241+21) ≈ (722, 262)。
    let action = hit_action(
        OriginalScreen::ChooseMap,
        &maps,
        0,
        None,
        (722.0, 262.0),
        800.0,
        600.0,
        0,
        false,
    );
    assert_eq!(action, Some(MenuAction::UseMap));

    let cancel = hit_action(
        OriginalScreen::ChooseMap,
        &maps,
        0,
        None,
        (722.0, 556.0),
        800.0,
        600.0,
        0,
        false,
    );
    assert_eq!(cancel, Some(MenuAction::Back));
}

#[test]
fn choose_map_mode_and_map_rows_are_hit() {
    let maps = vec![sample_map("mp01t4.map"), sample_map("mp03t4.map")];
    let hits = hits_for(OriginalScreen::ChooseMap, &maps, 2, 0, false);
    assert!(hits.iter().any(|h| matches!(h.action, MenuAction::SelectMode(0))));
    assert!(hits.iter().any(|h| matches!(h.action, MenuAction::SelectMap(0))));
    assert!(hits.iter().any(|h| matches!(h.action, MenuAction::SelectMap(1))));

    // game_type_list (30,127,195,260) 首行中心；map_list (252,127) 首行。
    assert_eq!(
        hit_action(OriginalScreen::ChooseMap, &maps, 2, None, (80.0, 135.0), 800.0, 600.0, 0, false),
        Some(MenuAction::SelectMode(0))
    );
    assert_eq!(
        hit_action(OriginalScreen::ChooseMap, &maps, 2, None, (300.0, 135.0), 800.0, 600.0, 0, false),
        Some(MenuAction::SelectMap(0))
    );
    assert_eq!(
        hit_action(OriginalScreen::ChooseMap, &maps, 2, None, (300.0, 151.0), 800.0, 600.0, 0, false),
        Some(MenuAction::SelectMap(1))
    );
}

#[test]
fn choose_map_scroll_shifts_hit_rows() {
    let maps: Vec<_> = (0..20).map(|i| sample_map(&format!("mp{i:02}t4.map"))).collect();
    assert_eq!(
        hit_action(OriginalScreen::ChooseMap, &maps, 1, None, (300.0, 135.0), 800.0, 600.0, 0, false),
        Some(MenuAction::SelectMap(0))
    );
    assert_eq!(
        hit_action(OriginalScreen::ChooseMap, &maps, 1, None, (300.0, 135.0), 800.0, 600.0, 3, false),
        Some(MenuAction::SelectMap(3))
    );
}
