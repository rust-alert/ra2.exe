//! 同命令流下状态摘要一致。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, BattleState};
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

#[test]
fn twin_worlds_same_command_stream_match_hash() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Russians".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 14,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
    let mk = || {
        let mut w = BattleState::new(GameEdition::Ra2, &rules, map.clone());
        let a = w.entity_id_at(0).expect("entity");
        let b = w.entity_id_at(1).expect("entity");
        assert!(w.clear_ecs_movement(a));
        assert!(w.clear_ecs_movement(b));
        assert!(w.set_ecs_speed(b, 0));
        w
    };
    let mut a = mk();
    let mut b = mk();
    assert_eq!(a.state_hash(), b.state_hash());
    let cmds = [GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 10 }, GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) }];
    for cmd in &cmds {
        a.push_command(cmd.clone());
        b.push_command(cmd.clone());
        a.advance_tick();
        b.advance_tick();
        assert_eq!(a.state_hash(), b.state_hash());
        assert_eq!(a.last_input_frame(), b.last_input_frame());
    }
    for _ in 0..20 {
        a.advance_tick();
        b.advance_tick();
        assert_eq!(a.state_hash(), b.state_hash());
    }
}
