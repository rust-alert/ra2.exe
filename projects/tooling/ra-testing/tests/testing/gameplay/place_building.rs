//! 经 Session 放置建筑。

use ra_map::MapEntityKind;
use ra_testing::{alpha_skirmish_v1, yard_open};
use ra_types::PlayerId;

#[test]
fn place_power_through_session_deducts_funds() {
    let mut case = yard_open();
    let slice = alpha_skirmish_v1();
    let before = case.session.expect_battle().world.house_funds(slice.human_house).expect("应有资金");
    case.produce_and_place(PlayerId(0), "GAPOWR", 6, 4);
    assert!(case.session.expect_battle().world.last_rejects().is_empty());
    assert_eq!(case.session.expect_battle().world.house_funds(slice.human_house), Some(before - 600));
    assert_eq!(case.session.expect_battle().world.entity_count(), 2);
    let id = case.session.expect_battle().world.entity_id_at(1).expect("placed power");
    let (type_id, kind) = case.session.expect_battle().world.ecs_identity(id).expect("power identity");
    assert_eq!(kind, MapEntityKind::Structure);
    assert_eq!(type_id.as_ref(), "GAPOWR");
    assert_eq!(case.session.expect_battle().world.players[0].power_output, 200);
}
