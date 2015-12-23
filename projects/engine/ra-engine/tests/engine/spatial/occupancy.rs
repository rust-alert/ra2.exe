//! 移动体互斥占格绕行。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, BattleState};
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::{EntityId, GameEdition};

#[test]
fn mobiles_detour_around_each_other() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.waypoints.push(Waypoint { index: 0, x: 14, y: 10 });
    // 挡在直线上的静止单位（Speed=0 用建筑外的占格：另一辆坦克无目标则不移动）。
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
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
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    // 两车都朝同一目标；后者路径不得踩前者当前格。
    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 14, y: 10 });
    world.push_command(GameCommand::MoveTo { entity: EntityId(2), x: 14, y: 10 });
    world.advance_tick();
    assert!(!world.ecs_path(world.entity_id_at(1).expect("entity")).expect("path").is_empty());
    assert!(!world.ecs_path(world.entity_id_at(1).expect("entity")).expect("path").iter().any(|&(x, y)| x == world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0 && y == world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").1));
    for _ in 0..40 {
        world.advance_tick();
    }
    // 至少一车抵达或贴近目标；且不同时占同一格。
    let a = (world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").1);
    let b = (world.ecs_transform(world.entity_id_at(1).expect("entity")).expect("xf").0, world.ecs_transform(world.entity_id_at(1).expect("entity")).expect("xf").1);
    assert_ne!(a, b);
    assert!(a == (14, 10) || b == (14, 10) || a.0.max(b.0) >= 13);
}
