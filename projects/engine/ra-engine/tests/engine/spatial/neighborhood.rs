//! 共享航点邻域排队。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{BattleState, GameCommand};
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::{EntityId, GameEdition};

#[test]
fn shared_waypoint_queues_on_neighbor() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.waypoints.push(Waypoint { index: 0, x: 12, y: 10 });
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
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 11,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 10 });
    world.push_command(GameCommand::MoveTo { entity: EntityId(2), x: 12, y: 10 });
    for _ in 0..30 {
        world.advance_tick();
    }
    let a = (
        world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0,
        world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").1,
    );
    let b = (
        world.ecs_transform(world.entity_id_at(1).expect("entity")).expect("xf").0,
        world.ecs_transform(world.entity_id_at(1).expect("entity")).expect("xf").1,
    );
    assert_ne!(a, b);
    // 一车占目标，另一车停在曼哈顿距离 ≤2 的邻域。
    let on_wp = |p: (u16, u16)| p == (12, 10);
    assert!(on_wp(a) || on_wp(b));
    let other = if on_wp(a) { b } else { a };
    let dist = (i32::from(other.0) - 12).unsigned_abs() + (i32::from(other.1) - 10).unsigned_abs();
    assert!(dist <= 2);
}
