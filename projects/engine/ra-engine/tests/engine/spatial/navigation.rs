//! 绕静态障碍寻路。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, BattleState};
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::{EntityId, GameEdition};

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
