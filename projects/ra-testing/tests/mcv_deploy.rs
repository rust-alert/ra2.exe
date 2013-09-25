//! MCV 部署经 Session 路径。

use ra_engine::GameCommand;
use ra_map::MapEntityKind;
use ra_testing::{alpha_skirmish_v1, mcv_deploy_open};

#[test]
fn mcv_deploy_open_seeds_slice_funds() {
    let mut case = mcv_deploy_open();
    let slice = alpha_skirmish_v1();
    assert_eq!(case.session.expect_game().world.house_funds(slice.human_house), Some(slice.starting_funds));
    assert_eq!(case.session.expect_game().world.entities[0].type_id, slice.allied_mcv);
}

#[test]
fn mcv_deploy_through_session_becomes_yard() {
    let mut case = mcv_deploy_open();
    let id = case.session.expect_game_mut().world.entities[0].id;
    case.command(GameCommand::Deploy { entity_index: 0 });
    case.advance(1);
    assert!(case.session.expect_game_mut().world.last_rejects().is_empty());
    assert_eq!(case.session.expect_game_mut().world.entities[0].id, id);
    assert_eq!(case.session.expect_game_mut().world.entities[0].kind, MapEntityKind::Structure);
    assert_eq!(case.session.expect_game_mut().world.entities[0].type_id, "GACNST");
    assert!(case.observe().outcome.is_none());
}
