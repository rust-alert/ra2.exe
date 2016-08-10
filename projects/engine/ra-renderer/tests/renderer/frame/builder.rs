//! `FrameBuilder` 与 `RenderWorld` 槽位复用。

use ra_engine::{AnimState, SnapshotUnit};
use ra_map::MapEntityKind;
use ra_renderer::{FrameBuilder, RenderWorld};
use ra_types::EntityId;
use std::sync::Arc;

fn unit(id: u64, x: i32, y: i32) -> SnapshotUnit {
    SnapshotUnit {
        id: EntityId(id),
        kind: MapEntityKind::Unit,
        type_id: Arc::from("MTNK"),
        owner: Arc::from("Americans"),
        x: 0,
        y: 0,
        screen_x: x,
        screen_y: y,
        facing: 0,
        turret_facing: 0,
        hva_frame: 0,
        anim_state: AnimState::Idle,
        health: 100,
        max_health: 100,
        dead: false,
        deployable: false,
        move_goal_screen: None,
        attack_target: None,
        attack_target_screen: None,
        foundation_w: 0,
        foundation_h: 0,
        art_height: 0,
        bracket_delta: 0,
    }
}

#[test]
fn dirty_update_reuses_slots_and_drops_missing() {
    let mut world = RenderWorld::default();
    let a = unit(1, 10, 20);
    let b = unit(2, 30, 40);
    FrameBuilder::apply_dirty_units(&mut world, 1, &[EntityId(1), EntityId(2)], &[a, b], &[]);
    assert_eq!(world.unit_count(), 2);
    assert_eq!(world.units.get(&1).unwrap().screen_x, 10);

    let a2 = unit(1, 11, 21);
    FrameBuilder::apply_dirty_units(&mut world, 2, &[EntityId(1), EntityId(2)], &[a2], &[]);
    assert_eq!(world.unit_count(), 1);
    assert_eq!(world.units.get(&1).unwrap().screen_x, 11);
    assert!(!world.units.contains_key(&2));
}

#[test]
fn selection_refresh_without_reprojecting_all() {
    let mut world = RenderWorld::default();
    let a = unit(1, 0, 0);
    let b = unit(2, 0, 0);
    FrameBuilder::apply_dirty_units(&mut world, 1, &[EntityId(1), EntityId(2)], &[a, b], &[EntityId(1)]);
    assert!(world.units.get(&1).unwrap().selected);
    assert!(!world.units.get(&2).unwrap().selected);

    FrameBuilder::apply_dirty_units(&mut world, 1, &[], &[], &[EntityId(2)]);
    assert!(!world.units.get(&1).unwrap().selected);
    assert!(world.units.get(&2).unwrap().selected);
}

#[test]
fn hover_refresh_without_reprojecting_all() {
    let mut world = RenderWorld::default();
    let a = unit(1, 0, 0);
    let b = unit(2, 0, 0);
    FrameBuilder::apply_dirty_units(&mut world, 1, &[EntityId(1), EntityId(2)], &[a, b], &[]);
    world.hover_id = Some(EntityId(1));
    FrameBuilder::apply_dirty_units(&mut world, 1, &[], &[], &[]);
    assert!(world.units.get(&1).unwrap().hovered);
    assert!(!world.units.get(&2).unwrap().hovered);

    world.hover_id = Some(EntityId(2));
    FrameBuilder::apply_dirty_units(&mut world, 1, &[], &[], &[]);
    assert!(!world.units.get(&1).unwrap().hovered);
    assert!(world.units.get(&2).unwrap().hovered);
}
