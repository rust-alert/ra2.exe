//! 移动命令推进。

use crate::common::{map_with_size, rules_with_mtnk};
use ra_engine::{GameCommand, MatchState};
use ra_map::{MapEntity, MapEntityKind, Waypoint};
use ra_types::GameEdition;

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
    });
    let mut world = MatchState::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.entities[0].target_x, None);
    assert!(world.entities[0].path.is_empty());
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 20 });
    world.advance_tick();
    assert_eq!(world.entities[0].target_x, Some(12));
    assert_eq!(world.entities[0].x, 11);
    assert_eq!(world.entities[0].hva_frame, 1);
    world.advance_tick();
    assert_eq!(world.entities[0].x, 12);
    assert_eq!(world.entities[0].hva_frame, 2);
    world.advance_tick();
    assert_eq!(world.entities[0].x, 12);
}

#[test]
fn move_to_command_sets_target() {
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
    });
    let mut world = MatchState::new(GameEdition::Ra2, &rules, map);
    assert_eq!(world.entities[0].target_x, None);
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 20 });
    world.advance_tick();
    assert_eq!(world.entities[0].target_x, Some(12));
    assert_eq!(world.entities[0].x, 11);
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
    });
    let mut world = MatchState::new(GameEdition::Ra2, &rules, map);
    world.entities[0].turret_facing = 128;
    world.push_command(GameCommand::MoveTo { entity_index: 0, x: 12, y: 20 });
    world.advance_tick();
    // 车身迈步后 facing 变；炮塔每 tick 最多转 TURRET_TURN_STEP。
    let body = world.entities[0].facing;
    let tur = world.entities[0].turret_facing;
    assert_ne!(tur, 128);
    let delta = (i16::from(body) - i16::from(tur)).rem_euclid(256);
    let shortest = if delta > 128 { 256 - delta } else { delta };
    assert!(shortest < 128);
}
