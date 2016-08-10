//! `ra-ecs` 行为测例。

use ra_ecs::{EcsEntity, EcsWorld};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Health(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Label(&'static str);

#[test]
fn spawn_contains_and_len() {
    let mut world = EcsWorld::new();
    assert!(world.is_empty());
    let a = world.spawn();
    let b = world.spawn();
    assert!(world.contains(a));
    assert!(world.contains(b));
    assert_eq!(world.len(), 2);
}

#[test]
fn despawn_invalidates_handle_and_reuses_slot() {
    let mut world = EcsWorld::new();
    let a = world.spawn();
    assert!(world.despawn(a));
    assert!(!world.contains(a));
    assert_eq!(world.len(), 0);

    let b = world.spawn();
    assert_eq!(b.slot, a.slot);
    assert_ne!(b.generation, a.generation);
    assert!(world.contains(b));
    assert!(!world.contains(a));
}

#[test]
fn insert_get_remove_components() {
    let mut world = EcsWorld::new();
    let e = world.spawn();
    assert_eq!(world.insert(e, Health(10)), None);
    assert_eq!(world.get::<Health>(e), Some(&Health(10)));
    assert_eq!(world.insert(e, Health(20)), Some(Health(10)));
    assert_eq!(world.remove::<Health>(e), Some(Health(20)));
    assert_eq!(world.get::<Health>(e), None);
}

#[test]
fn despawn_clears_components() {
    let mut world = EcsWorld::new();
    let e = world.spawn();
    world.insert(e, Health(3));
    world.insert(e, Label("a"));
    assert!(world.despawn(e));
    assert_eq!(world.get::<Health>(e), None);
    assert_eq!(world.get::<Label>(e), None);
}

#[test]
fn command_buffer_applies_in_order() {
    let mut world = EcsWorld::new();
    let mut commands = world.commands();
    commands.spawn_with(Health(1));
    commands.spawn_with(Health(2));
    world.apply(commands);
    assert_eq!(world.len(), 2);

    let mut values: Vec<_> = world.iter::<Health>().map(|(_, h)| h.0).collect();
    values.sort_unstable();
    assert_eq!(values, vec![1, 2]);
}

#[test]
fn command_buffer_despawn_and_insert() {
    let mut world = EcsWorld::new();
    let e = world.spawn();
    world.insert(e, Health(9));

    let mut commands = world.commands();
    commands.insert(e, Health(11));
    commands.remove::<Health>(e);
    commands.insert(e, Label("ok"));
    world.apply(commands);

    assert_eq!(world.get::<Health>(e), None);
    assert_eq!(world.get::<Label>(e), Some(&Label("ok")));

    let mut commands = world.commands();
    commands.despawn(e);
    world.apply(commands);
    assert!(!world.contains(e));
}

#[test]
fn stale_handle_ops_are_ignored() {
    let mut world = EcsWorld::new();
    let e = world.spawn();
    world.despawn(e);
    assert_eq!(world.insert(e, Health(1)), None);
    assert_eq!(world.remove::<Health>(e), None);
    assert_eq!(world.get::<Health>(e), None);
}

#[test]
fn clone_world_preserves_components() {
    let mut world = EcsWorld::new();
    let e = world.spawn();
    world.insert(e, Health(7));
    let cloned = world.clone();
    assert!(cloned.contains(e));
    assert_eq!(cloned.get::<Health>(e), Some(&Health(7)));
}

#[test]
fn entity_handle_is_copy() {
    let e = EcsEntity { slot: 1, generation: 2 };
    let f = e;
    assert_eq!(e, f);
}
