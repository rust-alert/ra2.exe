//! 快照动画状态派生。

use crate::common::{test_engine, rules_with_mtnk};
use ra_engine::{AnimState, GameCommand, Session, MatchState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn snapshot_anim_state_moves_when_ordered() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "anim");
    map.width = 20;
    map.height = 20;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    let mut session = Session::from_state(MatchState::new(GameEdition::Ra2, &rules, map), "anim");
    assert_eq!(session.expect_game().snapshot(&[]).units[0].anim_state, AnimState::Idle);
    session.expect_game_mut().push_command(GameCommand::MoveTo { entity: EntityId(1), x: 10, y: 4 });
    session.tick(&engine.runtime());
    assert_eq!(session.expect_game().snapshot(&[]).units[0].anim_state, AnimState::Move);
}
