//! MCV 部署经 Session 路径。

use ra_engine::GameCommand;
use ra_map::MapEntityKind;
use ra_testing::{alpha_skirmish_v1, mcv_deploy_open};

#[test]
fn mcv_deploy_open_seeds_slice_funds() {
    let case = mcv_deploy_open();
    let slice = alpha_skirmish_v1();
    assert_eq!(case.session.expect_battle().world.house_funds(slice.human_house), Some(slice.starting_funds));
    let id = case.session.expect_battle().world.entity_id_at(0).expect("seeded mcv");
    let (type_id, _) = case.session.expect_battle().world.ecs_identity(id).expect("mcv identity");
    assert_eq!(type_id.as_ref(), slice.allied_mcv);
}

#[test]
fn mcv_deploy_through_session_becomes_yard() {
    let mut case = mcv_deploy_open();
    let id = case.session.expect_battle().world.entity_id_at(0).expect("seeded mcv");
    case.command(GameCommand::Deploy { entity: id });
    case.advance(1);
    assert!(case.session.expect_battle().world.last_rejects().is_empty());
    assert_eq!(case.session.expect_battle().world.entity_id_at(0), Some(id));
    let (type_id, kind) = case.session.expect_battle().world.ecs_identity(id).expect("deployed yard");
    assert_eq!(kind, MapEntityKind::Structure);
    assert_eq!(type_id.as_ref(), "GACNST");
    assert!(case.observe().outcome.is_none());
}
