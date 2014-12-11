//! `SystemSchedule` 驱动 tick 阶段。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, MatchState, SystemPhase, SystemSchedule};
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

#[test]
fn omitting_combat_phase_skips_damage() {
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
    let a = world.entities[0].id;
    let b = world.entities[1].id;
    assert!(world.clear_ecs_movement(a));
    assert!(world.clear_ecs_movement(b));
    assert!(world.set_ecs_speed(b, 0));
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    let start_hp = world.entities[1].health;
    let schedule = SystemSchedule::standard().without(SystemPhase::Combat);
    assert!(!schedule.contains(SystemPhase::Combat));
    world.advance_scheduled_tick(&schedule);
    assert_eq!(world.entities[0].attack_target, Some(EntityId(2)));
    assert_eq!(world.entities[1].health, start_hp);
}
