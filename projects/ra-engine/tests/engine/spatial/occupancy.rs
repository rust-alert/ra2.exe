//! 移动体互斥占格绕行。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, MatchState};
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::GameEdition;

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
    });
    let mut world = MatchState::new(GameEdition::Ra2, &rules, map);
    // 两车都朝同一目标；后者路径不得踩前者当前格。
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 14, y: 10 });
    world.push_command(GameCommand::MoveTo { entity_index: 1, x: 14, y: 10 });
    world.advance_tick();
    assert!(!world.entities[1].path.is_empty());
    assert!(!world.entities[1].path.iter().any(|&(x, y)| x == world.entities[0].x && y == world.entities[0].y));
    for _ in 0..40 {
        world.advance_tick();
    }
    // 至少一车抵达或贴近目标；且不同时占同一格。
    let a = (world.entities[0].x, world.entities[0].y);
    let b = (world.entities[1].x, world.entities[1].y);
    assert_ne!(a, b);
    assert!(a == (14, 10) || b == (14, 10) || a.0.max(b.0) >= 13);
}
