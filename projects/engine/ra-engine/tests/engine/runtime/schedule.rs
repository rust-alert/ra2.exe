//! `SystemSchedule` 驱动 tick 阶段。

use crate::common::{battle_from_defs, defs_with_mtnk, map_with_size};
use ra_engine::{GameCommand, SystemPhase, SystemSchedule};
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

#[test]
fn default_order_runs_terrain_spawn_after_powers() {
    let order = SystemPhase::default_order();
    let powers = order.iter().position(|p| *p == SystemPhase::Powers).expect("Powers");
    let spawn = order.iter().position(|p| *p == SystemPhase::TerrainSpawn).expect("TerrainSpawn");
    let triggers = order.iter().position(|p| *p == SystemPhase::Triggers).expect("Triggers");
    assert!(powers < spawn, "TerrainSpawn must follow Powers");
    assert!(spawn < triggers, "TerrainSpawn must precede Triggers");
    assert!(SystemSchedule::standard().contains(SystemPhase::TerrainSpawn));
}

#[test]
fn omitting_combat_phase_skips_damage() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "RUSSIANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map);
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
