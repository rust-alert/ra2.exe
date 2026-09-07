//! 绕静态障碍寻路。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, World};
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::GameEdition;

#[test]
fn bfs_detours_around_structure() {
    let rules = rules_with_mtnk();
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
    let mut world = World::new(GameEdition::Ra2, &rules, map);
    assert!(!world.pass_grid.is_passable(12, 10));
    world.push_command(GameCommand::MoveTo { entity_index: 1, x: 14, y: 10 });
    world.advance_tick();
    assert!(!world.entities[1].path.is_empty());
    assert!(!world.entities[1].path.iter().any(|&(x, y)| x == 12 && y == 10));
    // 八邻绕行仍短于直线穿墙，且不踩封死格。
    assert!(!world.entities[1].path.is_empty());
    assert!(world.entities[1].path.len() >= 3);
    for _ in 0..20 {
        world.advance_tick();
    }
    assert_eq!(world.entities[1].x, 14);
    assert_eq!(world.entities[1].y, 10);
}
