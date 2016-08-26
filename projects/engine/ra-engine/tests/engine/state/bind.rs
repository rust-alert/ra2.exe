//! 规则绑定到实体运行时字段。

use crate::common::{battle_from_defs, defs_from_rules_ini, defs_with_mtnk, map_with_size};
use ra_engine::ATTACK_COOLDOWN_TICKS;
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{GameEdition, TechnoClass};

#[test]
fn binds_strength_and_speed() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 64,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.entity_count(), 1);
    let id = world.entity_id_at(0).expect("entity");
    let health = world.ecs_health(id).expect("health");
    let combat = world.ecs_combat_view(id).expect("combat");
    assert_eq!(health.1, 400);
    assert_eq!(health.0, 400);
    assert_eq!(world.ecs_speed(id), Some(64));
    assert_eq!(combat.attack_range, 6);
    assert_eq!(combat.attack_damage, 100);
    assert_eq!(combat.attack_cooldown_max, ATTACK_COOLDOWN_TICKS);
    assert_eq!(combat.techno_class, Some(TechnoClass::Vehicle));
    assert_eq!(world.bound_techno_count(), 1);
}

#[test]
fn unbound_techno_gets_zero_combat_stats() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "NOSUCH".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    let id = world.entity_id_at(0).expect("entity");
    let combat = world.ecs_combat_view(id).expect("combat");
    assert_eq!(combat.attack_range, 0);
    assert_eq!(combat.attack_damage, 0);
    assert_eq!(combat.attack_cooldown_max, 0);
    assert_eq!(combat.attack_verses, [0; 11]);
    assert!(combat.techno_class.is_none());
    assert_eq!(world.bound_techno_count(), 0);
}

#[test]
fn seeds_structure_health_from_map_ratio() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=CAGAS01\n\
[CAGAS01]\nStrength=1000\nSight=4\nCost=100\nArmor=wood\n",
    );
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "CAGAS01".into(),
        health: 64,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let world = battle_from_defs(GameEdition::Ra2, defs, map);
    let id = world.entity_id_at(0).expect("entity");
    let health = world.ecs_health(id).expect("health");
    assert_eq!(health.1, 1000);
    assert_eq!(health.0, 250, "64/256 of Strength=1000");
    assert!(!health.2);
}
