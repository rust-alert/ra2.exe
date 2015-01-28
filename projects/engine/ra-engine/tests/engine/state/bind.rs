//! 规则绑定到实体运行时字段。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_assets::TechnoKind;
use ra_engine::{ATTACK_COOLDOWN_TICKS, BattleState};
use ra_map::{MapEntity, MapEntityKind};
use ra_types::GameEdition;

#[test]
fn binds_strength_and_speed() {
    let rules = rules_with_mtnk();
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
    });
    let world = BattleState::new(GameEdition::Ra2, &rules, map);
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
    assert_eq!(combat.techno_kind, Some(TechnoKind::Vehicle));
    assert_eq!(world.bound_techno_count(), 1);
}

#[test]
fn unbound_techno_gets_zero_combat_stats() {
    let rules = rules_with_mtnk();
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
    });
    let world = BattleState::new(GameEdition::Ra2, &rules, map);
    let id = world.entity_id_at(0).expect("entity");
    let combat = world.ecs_combat_view(id).expect("combat");
    assert_eq!(combat.attack_range, 0);
    assert_eq!(combat.attack_damage, 0);
    assert_eq!(combat.attack_cooldown_max, 0);
    assert_eq!(combat.attack_verses, [0; 11]);
    assert!(combat.techno_kind.is_none());
    assert_eq!(world.bound_techno_count(), 0);
}
