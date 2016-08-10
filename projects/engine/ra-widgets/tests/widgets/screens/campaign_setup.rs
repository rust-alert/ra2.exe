//! 自 `engine/ra-widgets/src/screens/campaign_setup.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-widgets/src/screens/campaign_setup.rs :: tests
use ra_widgets::screens::campaign_setup::*;

#[test]
fn side_maps_to_stock_battle_ids() {
    assert_eq!(campaign_side_battle_id("allied"), Some("ALL1"));
    assert_eq!(campaign_side_battle_id("tutorial"), Some("TUT1"));
    assert_eq!(campaign_side_battle_id("soviet"), Some("SOV1"));
    assert_eq!(campaign_side_battle_id("yuri"), None);
}

#[test]
fn side_maps_to_load_houses() {
    assert_eq!(campaign_side_lobby_house("allied"), Some("Americans"));
    assert_eq!(campaign_side_lobby_house("tutorial"), Some("Americans"));
    assert_eq!(campaign_side_lobby_house("soviet"), Some("Russians"));
}

#[test]
fn difficulty_wraps_lobby_labels() {
    assert_eq!(campaign_difficulty_label(0), "Easy");
    assert_eq!(campaign_difficulty_label(1), "Normal");
    assert_eq!(campaign_difficulty_label(2), "Hard");
    assert_eq!(campaign_difficulty_label(3), "Easy");
}
