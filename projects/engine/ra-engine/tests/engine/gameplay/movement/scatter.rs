//! 散开：向邻近空闲格短距移动。

use crate::common::{battle_from_defs, defs_with_mtnk, map_with_size};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition, MissionKind};

#[test]
fn scatter_moves_stacked_units_to_nearby_cells() {
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
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    let a = world.entity_id_at(0).expect("a");
    let b = world.entity_id_at(1).expect("b");

    world.push_command(GameCommand::Scatter { entity: EntityId(1) });
    world.push_command(GameCommand::Scatter { entity: EntityId(2) });
    world.advance_tick();

    let dest_a = world.ecs_move_destination(a).expect("dest a");
    let dest_b = world.ecs_move_destination(b).expect("dest b");
    assert!(dest_a.0.is_some() && dest_a.1.is_some(), "scatter must set destination");
    assert!(dest_b.0.is_some() && dest_b.1.is_some(), "scatter must set destination");
    assert_ne!(dest_a, (Some(10), Some(10)), "must leave the stacked cell");
    assert_ne!(dest_b, (Some(10), Some(10)), "must leave the stacked cell");
    assert_eq!(world.ecs_mission(a), Some(MissionKind::Move));
    assert_eq!(world.ecs_mission(b), Some(MissionKind::Move));
}

#[test]
fn delete_destroys_owned_mobile() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    let id = world.entity_id_at(0).expect("unit");
    world.push_command(GameCommand::Delete { entity: EntityId(1) });
    world.advance_tick();
    assert!(world.ecs_health(id).expect("health").2, "Delete must kill owned unit");
}
