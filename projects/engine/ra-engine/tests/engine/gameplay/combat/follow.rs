//! 跟随：目的地钉住目标格；目标死亡后清跟随。

use crate::common::{battle_from_defs, defs_with_mtnk, map_with_size};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

#[test]
fn follow_tracks_target_cell_then_clears_on_death() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 14,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    let follower = world.entity_id_at(0).expect("follower");
    let target = world.entity_id_at(1).expect("target");

    world.push_command(GameCommand::Follow { entity: EntityId(1), target: EntityId(2) });
    world.advance_tick();

    assert_eq!(world.ecs_mission(follower), Some(ra_types::MissionKind::Follow), "Follow command must set mission");
    assert_eq!(world.ecs_follow_target(follower).expect("follow"), Some(target));
    assert_eq!(world.ecs_move_destination(follower).expect("dest"), (Some(14), Some(10)));

    // 目标挪格后，跟随方应改钉新目的地。
    assert!(world.set_ecs_cell(target, 16, 10));
    world.advance_tick();
    assert_eq!(world.ecs_move_destination(follower).expect("dest"), (Some(16), Some(10)));

    // 抵达同格后目的地应清空，避免每 tick 重寻路。
    assert!(world.set_ecs_cell(follower, 16, 10));
    world.advance_tick();
    assert_eq!(world.ecs_move_destination(follower).expect("dest"), (None, None));
    assert_eq!(world.ecs_mission(follower), Some(ra_types::MissionKind::Follow));
    assert_eq!(world.ecs_follow_target(follower).expect("follow"), Some(target));

    // 目标死亡后清跟随。
    assert!(world.set_ecs_health(target, 0, 256, true));
    world.advance_tick();
    assert_eq!(world.ecs_follow_target(follower).expect("follow"), None);
    assert_eq!(world.ecs_mission(follower), None);
}
