//! 快照动画状态派生。

use crate::common::{rules_with_mtnk, test_engine};
use ra_engine::{AnimState, BattleState, GameCommand, Session};
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
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "anim");
    assert_eq!(session.expect_battle().snapshot(&[]).units[0].anim_state, AnimState::Idle);
    session.expect_battle_mut().push_command(GameCommand::MoveTo { entity: EntityId(1), x: 10, y: 4 });
    session.tick(&engine.runtime());
    assert_eq!(session.expect_battle().snapshot(&[]).units[0].anim_state, AnimState::Move);
}
