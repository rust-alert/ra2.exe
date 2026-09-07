//! 单位互殴伤害与击杀。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, MatchState};
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

#[test]
fn attack_command_damages_and_kills() {
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
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
    });
    let mut world = MatchState::new(GameEdition::Ra2, &rules, map);
    // 取消航点游荡，专注开火。
    world.entities[0].target_x = None;
    world.entities[0].target_y = None;
    world.entities[1].target_x = None;
    world.entities[1].target_y = None;
    world.entities[1].speed = 0;
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    let start_hp = world.entities[1].health;
    world.advance_tick();
    assert_eq!(world.entities[0].attack_target, Some(1));
    assert!(world.entities[1].health < start_hp);
    for _ in 0..64 {
        world.advance_tick();
        if world.entities[1].dead {
            break;
        }
    }
    assert!(world.entities[1].dead);
    assert_eq!(world.entities[1].health, 0);
    assert_eq!(world.entities[0].attack_target, None);
}
