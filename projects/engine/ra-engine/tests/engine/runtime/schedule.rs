//! `SystemSchedule` 驱动 tick 阶段。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, BattleState, SystemPhase, SystemSchedule};
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
        mission: String::new(),
        tag: String::new(),
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
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    let a = world.entity_id_at(0).expect("entity");
    let b = world.entity_id_at(1).expect("entity");
    assert!(world.clear_ecs_movement(a));
    assert!(world.clear_ecs_movement(b));
    assert!(world.set_ecs_speed(b, 0));
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    let start_hp = world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0;
    let schedule = SystemSchedule::standard().without(SystemPhase::Combat);
    assert!(!schedule.contains(SystemPhase::Combat));
    world.advance_scheduled_tick(&schedule);
    assert_eq!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0, Some(EntityId(2)));
    assert_eq!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, start_hp);
}
