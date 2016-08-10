//! 移动命令推进。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{BattleState, GameCommand};
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::{EntityId, GameEdition};

#[test]
fn advances_when_ordered_to_move() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.ecs_move_destination(world.entity_id_at(0).expect("entity")).expect("dest").0, None);
    assert!(world.ecs_path(world.entity_id_at(0).expect("entity")).expect("path").is_empty());
    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 20 });
    world.advance_tick();
    assert_eq!(world.ecs_move_destination(world.entity_id_at(0).expect("entity")).expect("dest").0, Some(12));
    assert_eq!(world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, 11);
    assert_eq!(world.ecs_animation(world.entity_id_at(0).expect("entity")).expect("anim").0, 1);
    world.advance_tick();
    assert_eq!(world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, 12);
    assert_eq!(world.ecs_animation(world.entity_id_at(0).expect("entity")).expect("anim").0, 2);
    world.advance_tick();
    assert_eq!(world.ecs_transform(world.entity_id_at(0).expect("entity")).expect("xf").0, 12);
}

#[test]
fn move_path_queues_remaining_waypoints_and_advances() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    let id = world.entity_id_at(0).expect("entity");
    world.push_command(GameCommand::MovePath { entity: EntityId(1), points: vec![(12, 20), (12, 22)] });
    world.advance_tick();
    assert_eq!(world.ecs_move_destination(id).expect("dest"), (Some(12), Some(20)));
    assert_eq!(world.ecs_waypoints(id).expect("wp"), vec![(12, 22)]);
    // 走到首航点。
    world.advance_tick();
    assert_eq!(world.ecs_transform(id).expect("xf").0, 12);
    // 到达后切到下一航点。
    world.advance_tick();
    assert_eq!(world.ecs_move_destination(id).expect("dest"), (Some(12), Some(22)));
    assert!(world.ecs_waypoints(id).expect("wp").is_empty());
}

#[test]
fn turret_chases_body_facing() {
    let rules = rules_with_mtnk();
    let mut map = map_with_size();
    map.waypoints.push(Waypoint { index: 0, x: 12, y: 20 });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 20,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = BattleState::new(GameEdition::Ra2, &rules, map);
    let id0 = world.entity_id_at(0).expect("entity");
    assert!(world.set_ecs_turret_facing(id0, 128));
    world.push_command(GameCommand::MoveTo { entity: EntityId(1), x: 12, y: 20 });
    world.advance_tick();
    // 车身迈步后 facing 变；炮塔每 tick 最多转 TURRET_TURN_STEP。
    let body = world.ecs_transform(id0).expect("xf").2;
    let tur = world.ecs_turret_facing(id0).expect("turret");
    assert_ne!(tur, 128);
    let delta = (i16::from(body) - i16::from(tur)).rem_euclid(256);
    let shortest = if delta > 128 { 256 - delta } else { delta };
    assert!(shortest < 128);
}
