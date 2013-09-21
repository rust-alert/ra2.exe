//! 同命令流下状态摘要一致。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, World};
use ra_map::{MapEntity, MapEntityKind};
use ra_types::GameEdition;

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
        let mut w = World::new(GameEdition::Ra2, &rules, map.clone());
        w.entities[0].target_x = None;
        w.entities[0].target_y = None;
        w.entities[1].target_x = None;
        w.entities[1].target_y = None;
        w.entities[1].speed = 0;
        w
    };
    let mut a = mk();
    let mut b = mk();
    assert_eq!(a.state_hash(), b.state_hash());
    let cmds =
        [GameCommand::MoveTo { entity_index: 0, x: 12, y: 10 }, GameCommand::Attack { attacker_index: 0, target_index: 1 }];
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
