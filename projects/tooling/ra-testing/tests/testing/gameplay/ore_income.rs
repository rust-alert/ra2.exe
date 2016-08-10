//! 经 Session 建造扣款（矿车采矿入账另测）。

use ra_testing::{alpha_skirmish_v1, yard_open};
use ra_types::PlayerId;

#[test]
fn produce_and_place_power_and_refinery_deducts_funds() {
    let mut case = yard_open();
    let slice = alpha_skirmish_v1();
    case.produce_and_place(PlayerId(0), "GAPOWR", 6, 4);
    case.produce_and_place(PlayerId(0), "GAREFN", 8, 4);
    let after_build = case.session.expect_battle().world.house_funds(slice.human_house).expect("应有资金");
    assert_eq!(after_build, slice.starting_funds - 600 - 2000);
    assert!(case.session.expect_battle().world.find_entity_id_by_owner_type(slice.human_house, "GAREFN").is_some());
}
