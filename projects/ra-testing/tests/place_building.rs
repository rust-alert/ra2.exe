//! 经 Session 放置建筑。

use ra_engine::GameCommand;
use ra_map::MapEntityKind;
use ra_testing::{alpha_skirmish_v1, yard_open};
use ra_types::PlayerId;

#[test]
fn place_power_through_session_deducts_funds() {
    let mut case = yard_open();
    let slice = alpha_skirmish_v1();
    let before = case.session.expect_game_mut().world.house_funds(slice.human_house).expect("应有资金");
    case.command(GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 });
    case.advance(1);
    assert!(case.session.expect_game_mut().world.last_rejects().is_empty());
    assert_eq!(case.session.expect_game_mut().world.house_funds(slice.human_house), Some(before - 600));
    assert_eq!(case.session.expect_game_mut().world.entities.len(), 2);
    assert_eq!(case.session.expect_game_mut().world.entities[1].kind, MapEntityKind::Structure);
    assert_eq!(case.session.expect_game_mut().world.entities[1].type_id, "GAPOWR");
    assert_eq!(case.session.expect_game_mut().world.players[0].power_output, 200);
}
