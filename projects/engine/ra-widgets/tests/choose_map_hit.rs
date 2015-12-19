//! 选图命中改为消费 `0x6B` snapshot。

use ra_layout::{
    choose_map_list_row_rect, rect_px_from_snapshot, solve_choose_map, CHOOSE_MAP_LIST_ROW_H,
};
use ra_map::{BootMapCandidate, Theater};
use ra_widgets::{
    input::hit::{hit_action, hits_for},
    menu_action::MenuAction,
    original_screen::OriginalScreen,
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

fn list_row_center(list_id: &str, row: usize) -> (f64, f64) {
    let snap = solve_choose_map();
    let list = rect_px_from_snapshot(&snap, list_id);
    let row_rect = choose_map_list_row_rect(list, row, list.w);
    (
        (row_rect.x + row_rect.w / 2) as f64,
        (row_rect.y + CHOOSE_MAP_LIST_ROW_H / 2) as f64,
    )
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

    // 左栏表单已在内容区居中：点击点必须取自当前 snapshot，不能写死旧 DLU 像素。
    let (mode_x, mode_y) = list_row_center("game_type_list", 0);
    let (map0_x, map0_y) = list_row_center("map_list", 0);
    let (map1_x, map1_y) = list_row_center("map_list", 1);
    assert_eq!(
        hit_action(
            OriginalScreen::ChooseMap,
            &maps,
            2,
            None,
            (mode_x, mode_y),
            800.0,
            600.0,
            0,
            false
        ),
        Some(MenuAction::SelectMode(0))
    );
    assert_eq!(
        hit_action(
            OriginalScreen::ChooseMap,
            &maps,
            2,
            None,
            (map0_x, map0_y),
            800.0,
            600.0,
            0,
            false
        ),
        Some(MenuAction::SelectMap(0))
    );
    assert_eq!(
        hit_action(
            OriginalScreen::ChooseMap,
            &maps,
            2,
            None,
            (map1_x, map1_y),
            800.0,
            600.0,
            0,
            false
        ),
        Some(MenuAction::SelectMap(1))
    );
}

#[test]
fn choose_map_scroll_shifts_hit_rows() {
    let maps: Vec<_> = (0..20).map(|i| sample_map(&format!("mp{i:02}t4.map"))).collect();
    let (map_x, map_y) = list_row_center("map_list", 0);
    assert_eq!(
        hit_action(
            OriginalScreen::ChooseMap,
            &maps,
            1,
            None,
            (map_x, map_y),
            800.0,
            600.0,
            0,
            false
        ),
        Some(MenuAction::SelectMap(0))
    );
    assert_eq!(
        hit_action(
            OriginalScreen::ChooseMap,
            &maps,
            1,
            None,
            (map_x, map_y),
            800.0,
            600.0,
            3,
            false
        ),
        Some(MenuAction::SelectMap(3))
    );
}
