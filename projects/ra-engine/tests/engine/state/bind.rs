//! 规则绑定到实体运行时字段。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_assets::TechnoKind;
use ra_engine::{ATTACK_COOLDOWN_TICKS, MatchState};
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
    let world = MatchState::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.entities.len(), 1);
    let e = &world.entities[0];
    assert_eq!(e.max_health, 400);
    assert_eq!(e.health, 400);
    assert_eq!(e.speed, 64);
    assert_eq!(e.attack_range, 6);
    assert_eq!(e.attack_damage, 100);
    assert_eq!(e.attack_cooldown_max, ATTACK_COOLDOWN_TICKS);
    assert_eq!(e.techno_kind, Some(TechnoKind::Vehicle));
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
    let world = MatchState::new(GameEdition::Ra2, &rules, map);
    let e = &world.entities[0];
    assert_eq!(e.attack_range, 0);
    assert_eq!(e.attack_damage, 0);
    assert_eq!(e.attack_cooldown_max, 0);
    assert_eq!(e.attack_verses, [0; 11]);
    assert!(e.techno_kind.is_none());
    assert_eq!(world.bound_techno_count(), 0);
}
