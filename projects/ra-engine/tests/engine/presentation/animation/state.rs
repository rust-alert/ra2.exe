//! 快照动画状态派生。

use crate::common::rules_with_mtnk;
use ra_engine::{AnimState, GameCommand, Session, World};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn snapshot_anim_state_moves_when_ordered() {
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
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "anim");
    assert_eq!(session.snapshot().units[0].anim_state, AnimState::Idle);
    session.push_command(GameCommand::MoveTo { entity_index: 0, x: 10, y: 4 });
    session.tick();
    assert_eq!(session.snapshot().units[0].anim_state, AnimState::Move);
}
