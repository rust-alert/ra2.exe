//! 绕静态障碍寻路。

use crate::common::{map_with_size, defs_with_mtnk, battle_from_defs};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::{GameEdition, EntityId};

#[test]
fn bfs_detours_around_structure() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.waypoints.push(Waypoint { index: 0, x: 14, y: 10 });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "GAWALL".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: Default::default(),
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
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map);
    // 墙体属 Neutral，先入房主序会把本地玩家落在 Neutral；命令需切到美国人。
    assert!(world.prefer_local_house("Americans"));
    assert!(!world.pass_grid.is_passable(12, 10));
    world.push_command(GameCommand::MoveTo { entity: EntityId(2), x: 14, y: 10 });
    world.advance_tick();
    assert!(!world.ecs_path(world.entity_id_at(1).expect("entity")).expect("path").is_empty());
    assert!(!world.ecs_path(world.entity_id_at(1).expect("entity")).expect("path").iter().any(|&(x, y)| x == 12 && y == 10));
    // 八邻绕行仍短于直线穿墙，且不踩封死格。
    assert!(!world.ecs_path(world.entity_id_at(1).expect("entity")).expect("path").is_empty());
    assert!(world.ecs_path(world.entity_id_at(1).expect("entity")).expect("path").len() >= 3);
    for _ in 0..20 {
        world.advance_tick();
    }
    assert_eq!(world.ecs_transform(world.entity_id_at(1).expect("entity")).expect("xf").0, 14);
    assert_eq!(world.ecs_transform(world.entity_id_at(1).expect("entity")).expect("xf").1, 10);
}

// 自顶层 `spatial__navigation_footprint_tests.rs` 并入。

// 自 engine/ra-engine/src/spatial/navigation.rs :: footprint_tests
use ra_engine::spatial::navigation::{is_adjacent_to_footprint, manhattan_to_footprint, nearest_adjacent_to_footprint};

#[test]
fn footprint_distance_uses_nearest_cell() {
    // 2x2 锚点 (4,4) 覆盖 (4,4)(5,4)(4,5)(5,5)
    assert_eq!(manhattan_to_footprint(6, 4, 4, 4, 2, 2), 1);
    assert_eq!(manhattan_to_footprint(6, 5, 4, 4, 2, 2), 1);
    assert_eq!(manhattan_to_footprint(7, 4, 4, 4, 2, 2), 2);
    assert_eq!(manhattan_to_footprint(5, 4, 4, 4, 2, 2), 0);
    assert!(is_adjacent_to_footprint(6, 4, 4, 4, 2, 2));
    assert!(!is_adjacent_to_footprint(7, 4, 4, 4, 2, 2));
}

#[test]
fn nearest_adjacent_picks_closest_ring_cell() {
    assert_eq!(nearest_adjacent_to_footprint(7, 4, 4, 4, 2, 2), (6, 4));
    assert_eq!(nearest_adjacent_to_footprint(3, 4, 4, 4, 2, 2), (3, 4));
}
